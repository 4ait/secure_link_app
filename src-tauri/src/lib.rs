use crate::secure_link_client::{SecureLinkClient, SecureLinkClientError, SecureLinkClientState};
use log::warn;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri::{AppHandle, Manager, State};

mod secure_link_client;
#[cfg(feature = "secure-link-windows-service_manager")]
mod secure_link_windows_service_client;

#[cfg(feature = "secure-link-embedded-client")]
mod secure_link_embedded_client;

#[cfg(feature = "windows-registry")]
mod auth_token_windows_registry_storage;

pub static SECURE_LINK_APP_AUTH_TOKEN_KEY: &str = "secure-link-app:auth-token-key";

// Store menu items for direct updates
struct TrayMenuItems {
    connect_item: MenuItem<tauri::Wry>,
    disconnect_item: MenuItem<tauri::Wry>,
}

struct AppData {
    secure_link_client: Mutex<Option<Arc<dyn SecureLinkClient + Send + Sync>>>,
    tray_menu_items: Mutex<Option<TrayMenuItems>>,
    #[cfg(feature = "secure-link-windows-service_manager")]
    secure_link_service_log_file_path: std::path::PathBuf,
    #[cfg(not(feature = "windows-registry"))]
    auth_token_file_path: std::path::PathBuf,
    secure_link_server_host: String,
    secure_link_server_port: u16,
}

#[tauri::command]
async fn current_state(state: State<'_, AppData>) -> Result<String, String> {
    let maybe_client = ensure_secure_link_client_created(&state)
        .await
        .map_err(|e| format!("{:?}", e))?;

    if let Some(secure_link_client_locked) = maybe_client {
        let status = secure_link_client_locked
            .status()
            .await
            .map_err(|e| format!("{:?}", e))?;

        match status {
            SecureLinkClientState::Running => Ok("Running".to_string()),
            SecureLinkClientState::Pending => Ok("Pending".to_string()),
            SecureLinkClientState::Stopped => Ok("Stopped".to_string()),
        }
    } else {
        Ok("Stopped".to_string())
    }
}

#[cfg(feature = "secure-link-windows-service_manager")]
#[tauri::command]
async fn get_service_log(state: State<'_, AppData>) -> Result<String, String> {
    let log_file_path = state.secure_link_service_log_file_path.clone();

    if !log_file_path.exists() {
        return Ok("".to_string());
    }

    match std::fs::read_to_string(log_file_path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Не удалось прочитать файл логов: {}", e)),
    }
}

async fn ensure_secure_link_client_created(
    state: &State<'_, AppData>,
) -> Result<Option<Arc<dyn SecureLinkClient>>, Box<dyn std::error::Error>> {
    let auth_token = match load_auth_token(state)? {
        None => return Ok(None),
        Some(auth_token) => auth_token,
    };

    let secure_link_client = {
        let mut secure_link_client_locked = state.secure_link_client.lock().unwrap();

        match &mut *secure_link_client_locked {
            Some(secure_link_client) => secure_link_client.clone(),
            None => {
                #[cfg(feature = "secure-link-embedded-client")]
                let client = {
                    secure_link_embedded_client::SecureLinkEmbeddedClient::new(
                        &auth_token,
                        &state.secure_link_server_host,
                        state.secure_link_server_port,
                    )
                };

                #[cfg(feature = "secure-link-windows-service_manager")]
                let client = {
                    secure_link_windows_service_client::SecureLinkWindowsServiceClient::new(
                        &state.secure_link_server_host,
                        state.secure_link_server_port,
                        &auth_token,
                        &state.secure_link_service_log_file_path.to_str().unwrap(),
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

async fn reinitialize_secure_link_client(
    state: &State<'_, AppData>,
) -> Result<Option<Arc<dyn SecureLinkClient>>, Box<dyn std::error::Error>> {
    let current_client = { state.secure_link_client.lock().unwrap().clone() };

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
    let secure_link_client = ensure_secure_link_client_created(&state)
        .await
        .map_err(|e| format!("{:?}", e))?;

    if let Some(secure_link_client) = secure_link_client {
        match secure_link_client.start().await {
            Ok(()) => Ok(()),
            Err(SecureLinkClientError::UnauthorizedError) => Err(format!("UnauthorizedError")),
            Err(err) => Err(format!("{:?}", err)),
        }
    } else {
        Err(format!("No auth token"))
    }
}

#[tauri::command]
async fn stop(state: State<'_, AppData>) -> Result<(), String> {
    let maybe_client_clone = {
        state
            .secure_link_client
            .lock()
            .unwrap()
            .as_ref()
            .map(|client| client.clone())
    };

    if let Some(secure_link_client) = maybe_client_clone {
        secure_link_client
            .stop()
            .await
            .map_err(|e| format!("{:?}", e))?;
    }

    Ok(())
}

// Separate tray-specific start function
async fn tray_start(state: &State<'_, AppData>) -> Result<(), Box<dyn std::error::Error>> {
    let secure_link_client = ensure_secure_link_client_created(state).await?;

    if let Some(secure_link_client) = secure_link_client {
        match secure_link_client.start().await {
            Ok(()) => Ok(()),
            Err(SecureLinkClientError::UnauthorizedError) => {
                eprintln!("Unauthorized error when starting from tray");
                Err("Unauthorized error".into())
            }
            Err(err) => {
                eprintln!("Error starting from tray: {:?}", err);
                Err(err.into())
            }
        }
    } else {
        eprintln!("No auth token available for tray start");
        Err("No auth token".into())
    }
}

// Separate tray-specific stop function
async fn tray_stop(state: &State<'_, AppData>) -> Result<(), Box<dyn std::error::Error>> {
    let maybe_client_clone = {
        state
            .secure_link_client
            .lock()
            .unwrap()
            .as_ref()
            .map(|client| client.clone())
    };

    if let Some(secure_link_client) = maybe_client_clone {
        secure_link_client.stop().await?;
    }

    Ok(())
}

#[tauri::command]
async fn update_auth_token(state: State<'_, AppData>, auth_token: String) -> Result<(), String> {
    if let Some(current_auth_token) = load_auth_token(&state).map_err(|e| format!("{:?}", e))? {
        if current_auth_token == auth_token {
            return Ok(());
        }
    }

    store_auth_token(&state, auth_token).map_err(|e| format!("{:?}", e))?;

    reinitialize_secure_link_client(&state)
        .await
        .map_err(|e| format!("{:?}", e))?;

    Ok(())
}

#[tauri::command]
async fn get_auth_token(state: State<'_, AppData>) -> Result<Option<String>, String> {
    Ok(load_auth_token(&state).map_err(|e| e.to_string())?)
}

// Get current client state for tray updates
async fn get_client_state(state: &State<'_, AppData>) -> SecureLinkClientState {
    let maybe_client = match ensure_secure_link_client_created(state).await {
        Ok(client) => client,
        Err(_) => return SecureLinkClientState::Stopped,
    };

    if let Some(secure_link_client) = maybe_client {
        match secure_link_client.status().await {
            Ok(status) => status,
            Err(_) => SecureLinkClientState::Stopped,
        }
    } else {
        SecureLinkClientState::Stopped
    }
}

// Update tray menu items directly without recreating menu
async fn update_tray_menu(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let state = app.state::<AppData>();
    let client_state = get_client_state(&state).await;

    let (is_connect_enabled, is_disconnect_enabled) = match client_state {
        SecureLinkClientState::Stopped => (true, false),
        SecureLinkClientState::Pending | SecureLinkClientState::Running => (false, true),
    };

    // Update menu items directly
    let menu_items = state.tray_menu_items.lock().unwrap();
    if let Some(ref items) = *menu_items {
        items.connect_item.set_enabled(is_connect_enabled)?;
        items.disconnect_item.set_enabled(is_disconnect_enabled)?;
    }

    Ok(())
}

// Background task to poll and update tray menu
async fn tray_update_task(app: AppHandle) {
    let mut interval = tokio::time::interval(Duration::from_millis(200));

    loop {
        if let Err(e) = update_tray_menu(&app).await {
            eprintln!("Failed to update tray menu: {}", e);
        }

        interval.tick().await;
    }
}

#[cfg(feature = "windows-registry")]
fn load_auth_token(
    _state: &State<'_, AppData>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {

    Ok(auth_token_windows_registry_storage::load_auth_token()?)
}

#[cfg(not(feature = "windows-registry"))]
fn load_auth_token(
    state: &State<'_, AppData>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if let Ok(content) = std::fs::read_to_string(&state.auth_token_file_path) {
        if content.is_empty() {
            Ok(None)
        } else {
            Ok(Some(content))
        }
    } else {
        Ok(None)
    }
}

#[cfg(feature = "windows-registry")]
fn store_auth_token(
    _state: &State<'_, AppData>,
    auth_token: String,
) -> Result<(), Box<dyn std::error::Error>> {

    auth_token_windows_registry_storage::store_auth_token(&auth_token)?;

    Ok(())
}

#[cfg(not(feature = "windows-registry"))]
fn store_auth_token(
    state: &State<'_, AppData>,
    auth_token: String,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(&state.auth_token_file_path, auth_token)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .setup(move |app| {
            // Create menu items
            let show_item = MenuItem::with_id(app, "show", "Показать Secure Link", true, None::<&str>)?;
            let connect_item = MenuItem::with_id(app, "connect", "Подключиться", false, None::<&str>)?;
            let disconnect_item = MenuItem::with_id(app, "disconnect", "Отключиться", false, None::<&str>)?;
            let exit_item = MenuItem::with_id(app, "exit", "Закрыть Secure Link", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_item, &connect_item, &disconnect_item, &exit_item])?;

            // Store menu items for later updates
            let menu_items = TrayMenuItems {
                connect_item: connect_item.clone(),
                disconnect_item: disconnect_item.clone()
            };

            // Create tray icon and store the handle
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .tooltip("Secure Link")
                .on_menu_event(|app, event| {
                    let window = app.get_webview_window("main").unwrap();
                    let app_handle = app.clone();

                    match event.id.as_ref() {
                        "show" => {
                            window.show().unwrap();
                        }
                        "connect" => {
                            // Handle connect action using tray-specific function
                            tauri::async_runtime::spawn(async move {
                                let state = app_handle.state::<AppData>();
                                if let Err(e) = tray_start(&state).await {
                                    eprintln!("Failed to start secure link from tray: {}", e);
                                }
                            });
                        }
                        "disconnect" => {
                            // Handle disconnect action using tray-specific function
                            tauri::async_runtime::spawn(async move {
                                let state = app_handle.state::<AppData>();
                                if let Err(e) = tray_stop(&state).await {
                                    eprintln!("Failed to stop secure link from tray: {}", e);
                                }
                            });
                        }
                        "exit" => {
                            app.exit(0);
                        }
                        _ => {}
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
                tray_menu_items: Mutex::new(Some(menu_items)), // Store menu items

                #[cfg(feature = "secure-link-windows-service_manager")] secure_link_service_log_file_path: {
                    let service_log_file_name = "secure_link_service.log";
                    app_data_dir.join(&service_log_file_name)
                },
                #[cfg(not(feature = "windows-registry"))] auth_token_file_path: {
                    let auth_token_file = "auth_token_file.txt";
                    app_data_dir.join(&auth_token_file)
                },
                secure_link_server_host: secure_link_server_host.to_string(),
                secure_link_server_port
            });

            // Start the background tray update task
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tray_update_task(app_handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start,
            stop,
            current_state,
            update_auth_token,
            get_auth_token,
            #[cfg(feature = "secure-link-windows-service_manager")] get_service_log
        ])
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build()
        )
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
