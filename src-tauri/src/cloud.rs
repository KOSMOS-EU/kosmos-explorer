use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;

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
}

impl CloudState {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            clouds: Mutex::new(Vec::new()),
        }
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

// ── Tauri Commands ──

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

    // Sort: folders first, then by name
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
