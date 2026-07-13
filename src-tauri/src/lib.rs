pub mod cloud;
pub mod oidc;

use cloud::{
    cloud_add, cloud_get_user, cloud_list_files, cloud_list_spaces,
    cloud_load, cloud_remove, cloud_update_bearer, CloudState,
};
use oidc::{oidc_start, oidc_wait, OIDCState};
use tauri::Manager;
use std::sync::atomic::{AtomicU32, Ordering};

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(0);

#[tauri::command]
fn navigate_cloud(app: tauri::AppHandle, url: String) -> Result<(), String> {
    eprintln!("[Cloud] {}", url);
    let parsed: tauri::Url = url.parse().map_err(|e| format!("{}", e))?;
    let webview = app.get_webview("cloud").ok_or("Cloud WebView nicht gefunden")?;
    webview.navigate(parsed).map_err(|e| format!("{}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CloudState::new())
        .manage(OIDCState::new())
        .invoke_handler(tauri::generate_handler![
            cloud_load,
            cloud_add,
            cloud_remove,
            cloud_update_bearer,
            cloud_get_user,
            cloud_list_spaces,
            cloud_list_files,
            oidc_start,
            oidc_wait,
            navigate_cloud,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let left_w = 350.0;
            let win_w = 1200.0;
            let win_h = 800.0;

            let window = tauri::WindowBuilder::new(app, "main")
                .title("KOSMOS Explorer")
                .inner_size(win_w, win_h)
                .min_inner_size(800.0, 600.0)
                .decorations(true)
                .build()
                .expect("window failed");

            // Left: Local WebView (React app / tree)
            window.add_child(
                tauri::WebviewBuilder::new("local", tauri::WebviewUrl::App("index.html".into())),
                tauri::Position::Logical(tauri::LogicalPosition::new(0.0, 0.0)),
                tauri::Size::Logical(tauri::LogicalSize::new(left_w, win_h)),
            ).expect("local webview failed");

            // Right: Cloud WebView with on_new_window handler
            let app_handle = app.handle().clone();
            let cloud_wv = tauri::WebviewBuilder::new(
                "cloud",
                tauri::WebviewUrl::External("about:blank".parse().unwrap()),
            ).on_page_load(|wv, payload| {
                if payload.event() == tauri::webview::PageLoadEvent::Finished {
                    // Log window.open calls to stderr via title change
                    // Set folderviews extension preferences (localStorage-based, not server-synced)
                    // Inject extension preferences + desktop styles
                    wv.eval(concat!(
                        // Extension preferences
                        "try{var ep=JSON.parse(localStorage.getItem('extensionPreferences')||'{}');",
                        "ep['com.kosmos-eu.folderviews.app-new-window']={extensionPointId:'com.kosmos-eu.folderviews.app-new-window',selectedExtensionIds:['com.kosmos-eu.folderviews.app-new-window-enabled']};",
                        "ep['com.kosmos-eu.folderviews.app-compact']={extensionPointId:'com.kosmos-eu.folderviews.app-compact',selectedExtensionIds:['com.kosmos-eu.folderviews.app-compact-enabled']};",
                        "localStorage.setItem('extensionPreferences',JSON.stringify(ep));}catch(e){}",
                        // Desktop style: inject <style> tag AND use MutationObserver as fallback
                        "try{if(!document.getElementById('kosmos-desktop-style')){",
                        "var s=document.createElement('style');s.id='kosmos-desktop-style';",
                        "s.textContent='#mobile-nav{display:none!important}#web-nav-sidebar{display:none!important}';",
                        "(document.head||document.documentElement).appendChild(s);",
                        "}}catch(e){}",
                    )).ok();
                    eprintln!("[Inject] desktop style for {}", payload.url());
                }
            }).on_new_window(move |url, features| {
                eprintln!("[Cloud] NEW WINDOW: {}", url);
                let n = WINDOW_COUNTER.fetch_add(1, Ordering::Relaxed);
                let label = format!("doc-{}", n);

                // Extract app name and filename from URL path
                let title = url.path_segments()
                    .map(|segs| {
                        let parts: Vec<&str> = segs.collect();
                        let app = parts.first().unwrap_or(&"");
                        let file = parts.last().unwrap_or(&"");
                        let decoded_file = urlencoding::decode(file).unwrap_or((*file).into());
                        format!("{} — {}", decoded_file, app)
                    })
                    .unwrap_or_else(|| "KOSMOS Explorer".to_string());

                let mut builder = tauri::WebviewWindowBuilder::new(
                    &app_handle,
                    &label,
                    tauri::WebviewUrl::External("about:blank".parse().unwrap()),
                ).title(&title)
                 .inner_size(900.0, 700.0);

                // Linux: MUST set related_view for WebKit
                #[cfg(target_os = "linux")]
                {
                    builder = builder.with_related_view(features.opener().webview.clone());
                }

                match builder.build() {
                    Ok(win) => {
                        eprintln!("[Cloud] Created window: {}", label);
                        tauri::webview::NewWindowResponse::Create { window: win }
                    }
                    Err(e) => {
                        eprintln!("[Cloud] Window creation failed: {}", e);
                        tauri::webview::NewWindowResponse::Deny
                    }
                }
            });

            window.add_child(
                cloud_wv,
                tauri::Position::Logical(tauri::LogicalPosition::new(left_w, 0.0)),
                tauri::Size::Logical(tauri::LogicalSize::new(win_w - left_w, win_h)),
            ).expect("cloud webview failed");

            // Enable window.open() and clear cache in cloud WebView
            let cloud = app.get_webview("cloud").unwrap();
            cloud.with_webview(|wv| {
                #[cfg(target_os = "linux")]
                {
                    use webkit2gtk::{WebViewExt, SettingsExt, WebContextExt};
                    if let Some(settings) = wv.inner().settings() {
                        settings.set_javascript_can_open_windows_automatically(true);
                        eprintln!("[Setup] javascript_can_open_windows_automatically = true");
                    }
                    // Clear WebKit cache on startup so theme CSS is always fresh
                    if let Some(ctx) = wv.inner().context() {
                        ctx.clear_cache();
                        eprintln!("[Setup] WebKit cache cleared");
                    }
                }
            }).ok();

            eprintln!("[Setup] Local: {}x{}, Cloud: {}x{}", left_w, win_h, win_w - left_w, win_h);


            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
