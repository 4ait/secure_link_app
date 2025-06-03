use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use log::warn;
use crate::secure_link_client::{SecureLinkClient, SecureLinkClientError, SecureLinkClientState};

mod secure_link_client;
#[cfg(feature = "secure-link-windows-service_manager")]
mod secure_link_windows_service;

#[cfg(feature = "secure-link-embedded-client")]
mod secure_link_embedded_client;

pub static SECURE_LINK_APP_AUTH_TOKEN_KEY: &str = "secure-link-app:auth-token-key";

struct AppData {
    secure_link_client: Mutex<Option<Arc<dyn SecureLinkClient + Send + Sync>>>,
    #[cfg(feature = "secure-link-windows-service_manager")]
    secure_link_service_log_file_path: std::path::PathBuf,
    #[cfg(not(feature = "windows-credential-manager"))]
    auth_token_file_path: std::path::PathBuf,
    secure_link_server_host: String,
    secure_link_server_port: u16,
}

#[tauri::command]
async fn current_state(state: State<'_, AppData>) -> Result<String, String> {
    let maybe_client =
        ensure_secure_link_client_created(&state)
            .await
            .map_err(|e| format!("{:?}", e))?;

    if let Some(secure_link_client_locked) = maybe_client  {
        let status = secure_link_client_locked.status().await.map_err(|e| format!("{:?}", e))?;

        match status {
            SecureLinkClientState::Running => {
                Ok("Running".to_string())
            }
            SecureLinkClientState::Pending => {
                Ok("Pending".to_string())
            }
            SecureLinkClientState::Stopped => {
                Ok("Stopped".to_string())
            }
        }
    }
    else {
        Ok("Stopped".to_string())
    }
}

#[cfg(feature = "secure-link-windows-service_manager")]
#[tauri::command]
async fn get_service_log(state: State<'_, AppData>) -> Result<String, String> {
    let log_file_path = state.secure_link_service_log_file_path.clone();

    // Если файл не существует, возвращаем пустую строку
    if !log_file_path.exists() {
        return Ok("".to_string());
    }

    // Читаем содержимое файла логов
    match std::fs::read_to_string(log_file_path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Не удалось прочитать файл логов: {}", e))
    }
}

async fn ensure_secure_link_client_created(state: &State<'_, AppData>) -> Result<Option<Arc<dyn SecureLinkClient>>, Box<dyn std::error::Error>> {
    let auth_token =
        match load_auth_token(state)? {
            None => { return Ok(None)}
            Some(auth_token) => { auth_token }
        };

    let secure_link_client = {
        let mut secure_link_client_locked = state.secure_link_client.lock().unwrap();

        match &mut *secure_link_client_locked {
            Some(secure_link_client) => secure_link_client.clone(),
            None => {
                #[cfg(feature = "secure-link-embedded-client")]
                let client = {
                    secure_link_embedded_client::SecureLinkEmbeddedClient::new(&auth_token, &state.secure_link_server_host, state.secure_link_server_port)
                };

                #[cfg(feature = "secure-link-windows-service_manager")]
                let client = {
                    secure_link_windows_service::SecureLinkWindowsService::new(
                        &state.secure_link_server_host,
                        state.secure_link_server_port,
                        &auth_token,
                        &state.secure_link_service_log_file_path.to_str().unwrap()
                    )
                };

                let client_arc = Arc::new(client);
                *secure_link_client_locked = Some(client_arc.clone());
                client_arc
            }
        }
    };

    Ok(Some(secure_link_client))
}

async fn reinitialize_secure_link_client(state: &State<'_, AppData>) -> Result<Option<Arc<dyn SecureLinkClient>>, Box<dyn std::error::Error>> {
    let current_client = {
        state.secure_link_client.lock().unwrap().clone()
    };

    if let Some(client) = current_client {
        client.stop().await?;
    }

    {
        *state.secure_link_client.lock().unwrap() = None;
    };

    Ok(ensure_secure_link_client_created(state).await?)
}

#[tauri::command]
async fn start(state: State<'_, AppData>) -> Result<(), String> {
    let secure_link_client =
        ensure_secure_link_client_created(&state).await
            .map_err(|e| format!("{:?}", e))?;

    if let Some(secure_link_client) = secure_link_client {
        match secure_link_client.start().await {
            Ok(()) => Ok(()),
            Err(SecureLinkClientError::UnauthorizedError) => Err(format!("UnauthorizedError")),
            Err(err) => Err(format!("{:?}", err)),
        }
    }
    else {
        Err(format!("No auth token"))
    }
}

#[tauri::command]
async fn stop(state: State<'_, AppData>) -> Result<(), String> {
    let maybe_client_clone = {
        state.secure_link_client.lock().unwrap().as_ref().map(|client| { client.clone() })
    };

    if let Some(secure_link_client) = maybe_client_clone {
        secure_link_client.stop().await.map_err(|e| format!("{:?}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn update_auth_token(state: State<'_, AppData>, auth_token: String) -> Result<(), String> {
    if let Some(current_auth_token) = load_auth_token(&state).map_err(|e| format!("{:?}", e))?{
        if current_auth_token == auth_token {
            return Ok(())
        }
    }

    store_auth_token(&state, auth_token).map_err(|e| format!("{:?}", e))?;

    reinitialize_secure_link_client(&state).await
        .map_err(|e| format!("{:?}", e))?;

    Ok(())
}

#[tauri::command]
async fn get_auth_token(state: State<'_, AppData>) -> Result<Option<String>, String> {
    Ok(load_auth_token(&state).map_err(|e| e.to_string())?)
}

#[cfg(feature = "secure-link-windows-service_manager")]
#[tauri::command]
async fn reinstall_service() -> Result<(), String> {
    if secure_link_windows_service_manager::is_service_installed().map_err(|err|err.to_string())? {
        secure_link_windows_service_manager::uninstall_service()
            .map_err(|err|err.to_string())?;
    }

    secure_link_windows_service_manager::install_service(
        r#"E:\source\secure_link_windows_service\target\debug\secure_link_windows_service.exe"#
    ).map_err(|err|err.to_string())?;

    Ok(())
}

#[cfg(feature = "windows-credential-manager")]
fn load_auth_token(_state: &State<'_, AppData>) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match windows_credential_manager_rs::CredentialManager::load(SECURE_LINK_APP_AUTH_TOKEN_KEY) {
        Ok(Some(auth_token)) => Ok(Some(auth_token)),
        Ok(None) => Ok(None),
        Err(err) => Err(err)
    }
}

#[cfg(not(feature = "windows-credential-manager"))]
fn load_auth_token(state: &State<'_, AppData>) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if let Ok(content) = std::fs::read_to_string(&state.auth_token_file_path) {
        if content.is_empty() {
            Ok(None)
        }
        else {
            Ok(Some(content))
        }
    }
    else {
        Ok(None)
    }
}

#[cfg(feature = "windows-credential-manager")]
fn store_auth_token(_state: &State<'_, AppData>, auth_token: String) -> Result<(), Box<dyn std::error::Error>> {
    windows_credential_manager_rs::CredentialManager::store(SECURE_LINK_APP_AUTH_TOKEN_KEY, &auth_token)?;
    Ok(())
}

#[cfg(not(feature = "windows-credential-manager"))]
fn store_auth_token(state: &State<'_, AppData>, auth_token: String) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(&state.auth_token_file_path, auth_token)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .setup(move |app| {
            // Create tray menu
            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

            // Create tray icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .menu_on_left_click(false) // Disable menu on left click
                .tooltip("Secure Link")
                .on_menu_event(|app, event| {
                    let window = app.get_webview_window("main").unwrap();
                    match event.id.as_ref() {
                        "show" => {
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                        "hide" => {
                            window.hide().unwrap();
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|app, event| {
                    // Handle tray icon click events
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event {
                        let window = app.get_webview_window("main").unwrap();
                        if window.is_visible().unwrap() {
                            window.hide().unwrap();
                        } else {
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                    }
                })
                .build(app)?;

            let exe_path = std::env::current_exe()?;
            let exe_dir = exe_path.parent().ok_or("Failed to get parent directory of exe")?.to_path_buf();

            #[cfg(feature = "secure-link-windows-service_manager")]{
                if !secure_link_windows_service_manager::is_service_installed()? {
                    secure_link_windows_service_manager::install_service(
                        exe_dir.join("secure_link_windows_service.exe").to_str().unwrap()
                    )?;
                }
            }

            let app_data_dir = app.path().app_data_dir()?;

            let app_data_dir =
                if app_data_dir.exists() {
                    app_data_dir
                }
                else {
                    warn!("appdata dir not exists. using current binary location as appdata, ok for dev env.");
                    exe_dir
                };

            let secure_link_server_host = env!("SECURE_LINK_SERVER_HOST", "SECURE_LINK_SERVER_HOST not set");

            let secure_link_server_port = env!("SECURE_LINK_SERVER_PORT", "SECURE_LINK_SERVER_PORT not set").parse::<u16>()
                .expect("Invalid SECURE_LINK_SERVER_PORT number");

            app.manage(AppData {
                secure_link_client: Mutex::new(None),

                #[cfg(feature = "secure-link-windows-service_manager")] secure_link_service_log_file_path: {
                    let service_log_file_name = "secure_link_service.log";
                    app_data_dir.join(&service_log_file_name)
                },
                #[cfg(not(feature = "windows-credential-manager"))] auth_token_file_path: {
                    let auth_token_file = "auth_token_file.txt";
                    app_data_dir.join(&auth_token_file)
                },
                secure_link_server_host: secure_link_server_host.to_string(),
                secure_link_server_port
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start, 
            stop,
            current_state,
            update_auth_token,
            get_auth_token,
            #[cfg(feature = "secure-link-windows-service_manager")] get_service_log,
            #[cfg(feature = "secure-link-windows-service_manager")] reinstall_service
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}