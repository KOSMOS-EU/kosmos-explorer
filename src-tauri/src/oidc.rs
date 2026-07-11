use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use std::sync::Mutex;
use tokio::sync::oneshot;

#[derive(Debug, Deserialize)]
struct OIDCConfig {
    authorization_endpoint: String,
    token_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
}

const CLIENT_ID: &str = "OpenCloudDesktop";

async fn discover(client: &Client, base_url: &str) -> Result<OIDCConfig, String> {
    let url = format!("{}/.well-known/openid-configuration", base_url.trim_end_matches('/'));
    let resp = client.get(&url).send().await.map_err(|e| format!("OIDC Discovery: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("OIDC Discovery: HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| format!("OIDC JSON: {}", e))
}

fn generate_pkce() -> (String, String) {
    let mut buf = [0u8; 32];
    rand::rng().fill(&mut buf);
    let verifier = URL_SAFE_NO_PAD.encode(buf);
    let hash = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hash);
    (verifier, challenge)
}

fn generate_state() -> String {
    let mut buf = [0u8; 16];
    rand::rng().fill(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

pub struct OIDCState {
    pub rx: Mutex<Option<oneshot::Receiver<Result<String, String>>>>,
}

impl OIDCState {
    pub fn new() -> Self {
        Self { rx: Mutex::new(None) }
    }
}

/// Start OIDC flow: discover, PKCE, start callback server, return auth URL.
/// The callback server handles code exchange and sends the token via channel.
#[tauri::command]
pub async fn oidc_start(
    cloud_state: tauri::State<'_, crate::cloud::CloudState>,
    oidc_state: tauri::State<'_, OIDCState>,
    url: String,
) -> Result<String, String> {
    let client = cloud_state.client.clone();
    let config = discover(&client, &url).await?;
    let (verifier, challenge) = generate_pkce();
    let state = generate_state();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Callback-Server: {}", e))?;
    let port = listener.local_addr().unwrap().port();
    // Use 127.0.0.1 (not localhost) — Tauri intercepts "localhost" URLs
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        config.authorization_endpoint,
        CLIENT_ID,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("openid profile email"),
        &state,
        &challenge,
    );

    eprintln!("[OIDC] Auth URL: {}", auth_url);

    let (tx, rx) = oneshot::channel();
    {
        let mut guard = oidc_state.rx.lock().unwrap();
        *guard = Some(rx);
    }

    // Spawn callback server — handles the full flow
    let cb_state = state;
    let cb_verifier = verifier;
    let cb_token_endpoint = config.token_endpoint;
    let cb_redirect_uri = redirect_uri.clone();
    let cb_client = client;

    tokio::spawn(async move {
        let result = match listener.accept().await {
            Ok((stream, _)) => {
                handle_callback(stream, &cb_client, &cb_state, &cb_verifier, &cb_token_endpoint, &cb_redirect_uri).await
            }
            Err(e) => Err(format!("Accept: {}", e)),
        };
        let _ = tx.send(result);
    });

    Ok(auth_url)
}

async fn handle_callback(
    stream: tokio::net::TcpStream,
    client: &Client,
    expected_state: &str,
    verifier: &str,
    token_endpoint: &str,
    redirect_uri: &str,
) -> Result<String, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = [0u8; 4096];
    let mut stream = stream;
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");

    eprintln!("[OIDC] Callback: {}", path);

    let parsed = url::Url::parse(&format!("http://localhost{}", path))
        .map_err(|_| "URL parse error".to_string())?;
    let params: std::collections::HashMap<_, _> = parsed.query_pairs().collect();

    // Check for error
    if let Some(error) = params.get("error") {
        let desc = params.get("error_description").map(|s| s.to_string()).unwrap_or_default();
        let msg = format!("{}: {}", error, desc);
        eprintln!("[OIDC] Error: {}", msg);
        send_html(&mut stream, &format!("Fehler: {}", msg)).await;
        return Err(msg);
    }

    let code = params.get("code").ok_or("Kein Code")?;
    let state = params.get("state").map(|s| s.as_ref()).unwrap_or("");

    if state != expected_state {
        send_html(&mut stream, "Ungültiger State").await;
        return Err("State mismatch".to_string());
    }

    eprintln!("[OIDC] Got code, exchanging...");

    // Exchange code for token
    let form = [
        ("grant_type", "authorization_code"),
        ("client_id", CLIENT_ID),
        ("code", code.as_ref()),
        ("redirect_uri", redirect_uri),
        ("code_verifier", verifier),
    ];

    let resp = client.post(token_endpoint).form(&form).send().await
        .map_err(|e| format!("Token request: {}", e))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        eprintln!("[OIDC] Token exchange failed: {}", body);
        send_html(&mut stream, &format!("Token-Fehler: {}", body)).await;
        return Err(body);
    }

    let token: TokenResponse = resp.json().await
        .map_err(|e| format!("Token JSON: {}", e))?;

    eprintln!("[OIDC] Token OK!");
    send_html(&mut stream, "Anmeldung erfolgreich. Fenster wird geschlossen...").await;
    Ok(token.access_token)
}

async fn send_html(stream: &mut tokio::net::TcpStream, msg: &str) {
    use tokio::io::AsyncWriteExt;
    let html = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
        <html><body style='font-family:Segoe UI,sans-serif;display:flex;align-items:center;\
        justify-content:center;height:100vh;color:#333'><p>{}</p></body></html>",
        msg
    );
    let _ = stream.write_all(html.as_bytes()).await;
}

/// Wait for the OIDC flow to complete. Returns the access token.
#[tauri::command]
pub async fn oidc_wait(
    oidc_state: tauri::State<'_, OIDCState>,
) -> Result<String, String> {
    let rx = {
        let mut guard = oidc_state.rx.lock().unwrap();
        guard.take()
    };
    match rx {
        Some(rx) => rx.await.map_err(|_| "Login abgebrochen".to_string())?,
        None => Err("Kein Login aktiv".to_string()),
    }
}
