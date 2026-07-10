use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub name: String,
    pub url: String,
    pub token: Option<String>,
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
    pub oidc_rx: Mutex<Option<tokio::sync::oneshot::Receiver<Result<crate::oidc::TokenResponse, String>>>>,
}

impl CloudState {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            clouds: Mutex::new(Vec::new()),
            oidc_rx: Mutex::new(None),
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
    let url = format!("{}{}", base_url.trim_end_matches('/'), endpoint);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Verbindungsfehler: {}", e))?;

    if resp.status().as_u16() == 401 {
        return Err("sitzung_abgelaufen".to_string());
    }
    if !resp.status().is_success() {
        return Err(format!("API-Fehler {}", resp.status()));
    }

    resp.json()
        .await
        .map_err(|e| format!("JSON-Fehler: {}", e))
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
        token: None,
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
pub fn cloud_update_token(
    state: State<'_, CloudState>,
    app: AppHandle,
    index: usize,
    token: String,
) -> Vec<CloudConfig> {
    let mut clouds = state.clouds.lock().unwrap();
    if let Some(cloud) = clouds.get_mut(index) {
        cloud.token = Some(token);
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
    let spaces = data["value"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|d| Space {
            id: d["id"].as_str().unwrap_or("").to_string(),
            name: d["name"].as_str().unwrap_or("").to_string(),
            drive_type: d["driveType"].as_str().unwrap_or("").to_string(),
        })
        .collect();
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
    let endpoint = if path.is_empty() || path == "/" {
        format!("/graph/v1.0/drives/{}/items/root/children", space_id)
    } else {
        let clean = path.trim_start_matches('/');
        format!(
            "/graph/v1.0/drives/{}/items/root:/{clean}:/children",
            space_id
        )
    };

    let data = api_get(&state.client, &url, &token, &endpoint).await?;
    let mut items: Vec<FileItem> = data["value"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|v| FileItem {
            id: v["id"].as_str().unwrap_or("").to_string(),
            name: v["name"].as_str().unwrap_or("").to_string(),
            size: v["size"].as_u64().unwrap_or(0),
            mime_type: v["file"]["mimeType"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            is_folder: v["folder"].is_object(),
            last_modified: v["lastModifiedDateTime"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        })
        .collect();

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

    Ok(items)
}
