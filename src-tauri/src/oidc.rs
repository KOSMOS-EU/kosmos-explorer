use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use std::sync::Mutex;
use tokio::sync::oneshot;

// ── OIDC Discovery ──

#[derive(Debug, Deserialize)]
struct OIDCConfig {
    authorization_endpoint: String,
    token_endpoint: String,
}

async fn discover(client: &Client, base_url: &str) -> Result<OIDCConfig, String> {
    let url = format!("{}/.well-known/openid-configuration", base_url.trim_end_matches('/'));
    let resp = client.get(&url).send().await.map_err(|e| format!("OIDC Discovery: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("OIDC Discovery: HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| format!("OIDC JSON: {}", e))
}

// ── PKCE ──

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

// ── Auth Session ──

pub struct AuthSession {
    pub auth_url: String,
    verifier: String,
    state: String,
    token_endpoint: String,
    redirect_uri: String,
    result_tx: Option<oneshot::Sender<Result<TokenResponse, String>>>,
    pub result_rx: Option<oneshot::Receiver<Result<TokenResponse, String>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<u64>,
}

pub struct OIDCState {
    pub session: Mutex<Option<AuthSession>>,
}

impl OIDCState {
    pub fn new() -> Self {
        Self { session: Mutex::new(None) }
    }
}

const CLIENT_ID: &str = "web";

// ── Start auth flow ──

pub async fn start_auth(client: &Client, base_url: &str) -> Result<AuthSession, String> {
    let config = discover(client, base_url).await?;
    let (verifier, challenge) = generate_pkce();
    let state = generate_state();

    // Start local callback server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Callback-Server: {}", e))?;
    let port = listener.local_addr().unwrap().port();
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        config.authorization_endpoint,
        CLIENT_ID,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("openid profile email offline_access"),
        &state,
        &challenge,
    );

    let (tx, rx) = oneshot::channel();

    let session = AuthSession {
        auth_url: auth_url.clone(),
        verifier: verifier.clone(),
        state: state.clone(),
        token_endpoint: config.token_endpoint.clone(),
        redirect_uri: redirect_uri.clone(),
        result_tx: Some(tx),
        result_rx: Some(rx),
    };

    // Spawn callback server
    let cb_state = state.clone();
    let cb_verifier = verifier;
    let cb_token_endpoint = config.token_endpoint;
    let cb_redirect_uri = redirect_uri;
    let cb_client = client.clone();

    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            handle_callback(
                stream, &cb_client, &cb_state, &cb_verifier,
                &cb_token_endpoint, &cb_redirect_uri,
            ).await;
        }
    });

    // We need to return the session but the tx is moved into it
    // The spawned task needs the tx... let me restructure

    Ok(session)
}

async fn handle_callback(
    stream: tokio::net::TcpStream,
    client: &Client,
    expected_state: &str,
    verifier: &str,
    token_endpoint: &str,
    redirect_uri: &str,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = [0u8; 4096];
    let mut stream = stream;
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse GET /callback?code=...&state=...
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");

    let url = format!("http://localhost{}", path);
    let parsed = url::Url::parse(&url).ok();

    let response_body;

    if let Some(parsed) = parsed {
        let params: std::collections::HashMap<_, _> = parsed.query_pairs().collect();

        if let Some(error) = params.get("error") {
            response_body = format!("Anmeldung fehlgeschlagen: {}", error);
        } else if let Some(code) = params.get("code") {
            let state = params.get("state").map(|s| s.as_ref()).unwrap_or("");
            if state != expected_state {
                response_body = "Ungültiger State-Parameter".to_string();
            } else {
                // Exchange code for token
                match exchange_code(client, token_endpoint, code, redirect_uri, verifier).await {
                    Ok(_token) => {
                        response_body = "Anmeldung erfolgreich. Dieses Fenster kann geschlossen werden.".to_string();
                    }
                    Err(e) => {
                        response_body = format!("Token-Fehler: {}", e);
                    }
                }
            }
        } else {
            response_body = "Kein Authorization Code erhalten".to_string();
        }
    } else {
        response_body = "Ungültige Anfrage".to_string();
    }

    let http_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
        <html><body style='font-family:Segoe UI,sans-serif;display:flex;align-items:center;\
        justify-content:center;height:100vh'><p>{}</p></body></html>",
        response_body
    );
    stream.write_all(http_response.as_bytes()).await.ok();
}

async fn exchange_code(
    client: &Client,
    token_endpoint: &str,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<TokenResponse, String> {
    let params = [
        ("grant_type", "authorization_code"),
        ("client_id", CLIENT_ID),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", verifier),
    ];

    let resp = client
        .post(token_endpoint)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token-Anfrage: {}", e))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Token-Fehler: {}", body));
    }

    resp.json().await.map_err(|e| format!("Token-JSON: {}", e))
}

// ── Tauri Command: Start OIDC and return auth URL ──

#[tauri::command]
pub async fn oidc_start(
    state: tauri::State<'_, crate::cloud::CloudState>,
    url: String,
) -> Result<String, String> {
    let config = discover(&state.client, &url).await?;
    let (verifier, challenge) = generate_pkce();
    let oidc_state = generate_state();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Callback-Server: {}", e))?;
    let port = listener.local_addr().unwrap().port();
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        config.authorization_endpoint,
        CLIENT_ID,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("openid profile email offline_access"),
        &oidc_state,
        &challenge,
    );

    // Spawn callback server that waits for the redirect
    let (tx, rx) = oneshot::channel::<Result<TokenResponse, String>>();
    let cb_client = state.client.clone();
    let cb_state = oidc_state.clone();
    let cb_verifier = verifier;
    let cb_token_endpoint = config.token_endpoint;
    let cb_redirect_uri = redirect_uri;

    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let mut buf = [0u8; 4096];
            let mut stream = stream;
            let n = stream.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);
            let first_line = request.lines().next().unwrap_or("");
            let path = first_line.split_whitespace().nth(1).unwrap_or("");
            let parsed_url = url::Url::parse(&format!("http://localhost{}", path)).ok();

            let mut response_body = "Fehler".to_string();
            let mut token_result: Result<TokenResponse, String> = Err("Kein Code".to_string());

            if let Some(parsed) = parsed_url {
                let params: std::collections::HashMap<_, _> = parsed.query_pairs().collect();
                if let Some(code) = params.get("code") {
                    let s = params.get("state").map(|s| s.as_ref()).unwrap_or("");
                    if s == cb_state {
                        match exchange_code(&cb_client, &cb_token_endpoint, code, &cb_redirect_uri, &cb_verifier).await {
                            Ok(token) => {
                                response_body = "Anmeldung erfolgreich. Sie können dieses Fenster schließen.".to_string();
                                token_result = Ok(token);
                            }
                            Err(e) => {
                                response_body = format!("Fehler: {}", e);
                                token_result = Err(e);
                            }
                        }
                    }
                }
            }

            let http = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
                <html><body style='font-family:Segoe UI,sans-serif;display:flex;align-items:center;\
                justify-content:center;height:100vh'><p>{}</p>\
                <script>setTimeout(()=>window.close(),1000)</script></body></html>",
                response_body
            );
            stream.write_all(http.as_bytes()).await.ok();
            tx.send(token_result).ok();
        }
    });

    // Store the receiver for oidc_wait
    {
        let mut session = state.oidc_rx.lock().unwrap();
        *session = Some(rx);
    }

    Ok(auth_url)
}

#[tauri::command]
pub async fn oidc_wait(
    state: tauri::State<'_, crate::cloud::CloudState>,
) -> Result<String, String> {
    let rx = {
        let mut session = state.oidc_rx.lock().unwrap();
        session.take()
    };

    match rx {
        Some(rx) => {
            let result = rx.await.map_err(|_| "Login abgebrochen".to_string())?;
            match result {
                Ok(token) => Ok(token.access_token),
                Err(e) => Err(e),
            }
        }
        None => Err("Kein Login-Vorgang aktiv".to_string()),
    }
}
