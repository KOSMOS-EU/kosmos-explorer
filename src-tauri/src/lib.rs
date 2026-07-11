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
                .decorations(false)
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
                    wv.eval(
                        "const _wo=window.open;window.open=function(...a){document.title='[WO] '+JSON.stringify(a);return _wo.apply(this,a);};"
                    ).ok();
                }
            }).on_new_window(move |url, features| {
                eprintln!("[Cloud] NEW WINDOW: {}", url);
                let n = WINDOW_COUNTER.fetch_add(1, Ordering::Relaxed);
                let label = format!("doc-{}", n);

                // Create new window with related_view from opener
                let mut builder = tauri::WebviewWindowBuilder::new(
                    &app_handle,
                    &label,
                    tauri::WebviewUrl::External("about:blank".parse().unwrap()),
                ).title("KOSMOS Explorer")
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

            eprintln!("[Setup] Local: {}x{}, Cloud: {}x{}", left_w, win_h, win_w - left_w, win_h);


            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
