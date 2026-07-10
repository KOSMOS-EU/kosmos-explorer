pub mod cloud;
pub mod oidc;

use cloud::{
    cloud_add, cloud_get_user, cloud_list_files, cloud_list_spaces,
    cloud_load, cloud_remove, cloud_update_token, CloudState,
};
use oidc::{oidc_start, oidc_wait};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CloudState::new())
        .invoke_handler(tauri::generate_handler![
            cloud_load,
            cloud_add,
            cloud_remove,
            cloud_update_token,
            cloud_get_user,
            cloud_list_spaces,
            cloud_list_files,
            oidc_start,
            oidc_wait,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
