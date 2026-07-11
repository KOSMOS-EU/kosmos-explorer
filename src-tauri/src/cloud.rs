use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use urlencoding;
use tauri::{AppHandle, Manager, State};

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub name: String,
    pub url: String,
    pub bearer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    #[serde(rename = "driveType")]
    pub drive_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    pub id: String,
    pub name: String,
    pub size: u64,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "isFolder")]
    pub is_folder: bool,
    #[serde(rename = "lastModified")]
    pub last_modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub name: String,
}

// ── State ──

pub struct CloudState {
    pub client: Client,
    pub clouds: Mutex<Vec<CloudConfig>>,
}

impl CloudState {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            clouds: Mutex::new(Vec::new()),
        }
    }
}

// ── Persistence ──

fn config_path(app: &AppHandle) -> PathBuf {
    let dir = app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
    fs::create_dir_all(&dir).ok();
    dir.join("clouds.json")
}

fn load_clouds_from_disk(app: &AppHandle) -> Vec<CloudConfig> {
    let path = config_path(app);
    match fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_clouds_to_disk(app: &AppHandle, clouds: &[CloudConfig]) {
    let path = config_path(app);
    if let Ok(data) = serde_json::to_string_pretty(clouds) {
        fs::write(path, data).ok();
    }
}

// ── API helpers ──

async fn api_get(
    client: &Client,
    base_url: &str,
    token: &str,
    endpoint: &str,
) -> Result<serde_json::Value, String> {
    let url_str = format!("{}{}", base_url.trim_end_matches('/'), endpoint);
    eprintln!("[API] GET {}", url_str);
    let url = reqwest::Url::parse(&url_str).map_err(|e| format!("URL-Fehler: {}", e))?;
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Verbindungsfehler: {}", e))?;

    if resp.status().as_u16() == 401 {
        return Err("sitzung_abgelaufen".to_string());
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        eprintln!("[API] Error {} for {}: {}", status, url_str, body);
        return Err(format!("API-Fehler {}", status));
    }

    serde_json::from_str(&body).map_err(|e| format!("JSON-Fehler: {}", e))
}

// ── Tauri Commands: Cloud Management ──

#[tauri::command]
pub fn cloud_load(state: State<'_, CloudState>, app: AppHandle) -> Vec<CloudConfig> {
    let from_disk = load_clouds_from_disk(&app);
    let mut clouds = state.clouds.lock().unwrap();
    *clouds = from_disk.clone();
    from_disk
}

#[tauri::command]
pub fn cloud_add(
    state: State<'_, CloudState>,
    app: AppHandle,
    name: String,
    url: String,
) -> Vec<CloudConfig> {
    let mut clouds = state.clouds.lock().unwrap();
    clouds.push(CloudConfig {
        name,
        url: url.trim_end_matches('/').to_string(),
        bearer: None,
    });
    save_clouds_to_disk(&app, &clouds);
    clouds.clone()
}

#[tauri::command]
pub fn cloud_remove(
    state: State<'_, CloudState>,
    app: AppHandle,
    index: usize,
) -> Vec<CloudConfig> {
    let mut clouds = state.clouds.lock().unwrap();
    if index < clouds.len() {
        clouds.remove(index);
        save_clouds_to_disk(&app, &clouds);
    }
    clouds.clone()
}

#[tauri::command]
pub fn cloud_update_bearer(
    state: State<'_, CloudState>,
    app: AppHandle,
    index: usize,
    token: String,
) -> Vec<CloudConfig> {
    let mut clouds = state.clouds.lock().unwrap();
    if let Some(cloud) = clouds.get_mut(index) {
        cloud.bearer = Some(token);
        save_clouds_to_disk(&app, &clouds);
    }
    clouds.clone()
}

// ── Tauri Commands: API ──

#[tauri::command]
pub async fn cloud_get_user(
    state: State<'_, CloudState>,
    url: String,
    token: String,
) -> Result<UserInfo, String> {
    let data = api_get(&state.client, &url, &token, "/graph/v1.0/me").await?;
    let name = data["displayName"]
        .as_str()
        .or(data["mail"].as_str())
        .unwrap_or("Unbekannt")
        .to_string();
    Ok(UserInfo { name })
}

#[tauri::command]
pub async fn cloud_list_spaces(
    state: State<'_, CloudState>,
    url: String,
    token: String,
) -> Result<Vec<Space>, String> {
    let data = api_get(&state.client, &url, &token, "/graph/v1.0/me/drives").await?;
    let mut spaces: Vec<Space> = data["value"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|d| Space {
            id: d["id"].as_str().unwrap_or("").to_string(),
            name: d["name"].as_str().unwrap_or("").to_string(),
            drive_type: d["driveType"].as_str().unwrap_or("").to_string(),
        })
        .collect();
    // Personal first, then alphabetical
    spaces.sort_by(|a, b| {
        let a_personal = a.drive_type == "personal";
        let b_personal = b.drive_type == "personal";
        if a_personal != b_personal {
            return if a_personal { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater };
        }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });
    Ok(spaces)
}

#[tauri::command]
pub async fn cloud_list_files(
    state: State<'_, CloudState>,
    url: String,
    token: String,
    space_id: String,
    path: String,
) -> Result<Vec<FileItem>, String> {
    // Use WebDAV PROPFIND instead of Graph API (Graph returns 404 for OpenCloudDesktop client)
    let safe_id = space_id.replace('$', "%24");
    let dav_path = if path.is_empty() || path == "/" {
        format!("/dav/spaces/{}/", safe_id)
    } else {
        let clean = path.trim_start_matches('/').trim_end_matches('/');
        format!("/dav/spaces/{}/{}/", safe_id, clean)
    };

    let dav_url = format!("{}{}", url.trim_end_matches('/'), dav_path);
    eprintln!("[WebDAV] PROPFIND {}", dav_url);

    let propfind_body = r#"<?xml version="1.0"?>
<d:propfind xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
  <d:prop>
    <d:displayname/>
    <d:resourcetype/>
    <d:getcontentlength/>
    <d:getlastmodified/>
    <d:getetag/>
    <d:getcontenttype/>
    <oc:fileid/>
    <oc:size/>
  </d:prop>
</d:propfind>"#;

    let parsed_url = reqwest::Url::parse(&dav_url).map_err(|e| format!("URL-Fehler: {}", e))?;
    let resp = state.client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), parsed_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/xml; charset=utf-8")
        .header("Depth", "1")
        .body(propfind_body)
        .send()
        .await
        .map_err(|e| format!("WebDAV-Fehler: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if status.as_u16() == 401 {
        return Err("sitzung_abgelaufen".to_string());
    }
    if !status.is_success() && status.as_u16() != 207 {
        eprintln!("[WebDAV] Error {}: {}", status, body);
        return Err(format!("WebDAV-Fehler {}", status));
    }

    // Parse WebDAV XML response
    let mut items = Vec::new();
    let mut is_first = true; // Skip first response (the folder itself)

    for response_block in body.split("<d:response>").skip(1) {
        if is_first { is_first = false; continue; }

        let href = extract_xml(response_block, "d:href").unwrap_or_default();
        let name = href.split('/').filter(|s| !s.is_empty()).last()
            .map(|s| urlencoding::decode(s).unwrap_or_default().to_string())
            .unwrap_or_default();

        if name.is_empty() || name.starts_with('.') { continue; }

        let is_folder = response_block.contains("<d:collection/>");
        let size_str = extract_xml(response_block, "d:getcontentlength")
            .or_else(|| extract_xml(response_block, "oc:size"))
            .unwrap_or_default();
        let size = size_str.parse::<u64>().unwrap_or(0);
        let last_modified = extract_xml(response_block, "d:getlastmodified").unwrap_or_default();
        let content_type = extract_xml(response_block, "d:getcontenttype").unwrap_or_default();
        let file_id = extract_xml(response_block, "oc:fileid").unwrap_or_default();

        items.push(FileItem {
            id: file_id,
            name,
            size,
            mime_type: content_type,
            is_folder,
            last_modified,
        });
    }

    items.sort_by(|a, b| {
        if a.is_folder != b.is_folder {
            return if a.is_folder {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });

    eprintln!("[WebDAV] {} items", items.len());
    Ok(items)
}

fn extract_xml(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml.find(&close)?;
    if start < end {
        Some(xml[start..end].to_string())
    } else {
        None
    }
}
