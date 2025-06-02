use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

use crate::secure_link_client::{SecureLinkClient, SecureLinkClientError};

mod secure_link_client;
#[cfg(feature = "secure-link-windows-service_manager")]
mod secure_link_windows_service;

#[cfg(feature = "secure-link-embedded-client")]
mod secure_link_embedded_client;

pub static SECURE_LINK_APP_AUTH_TOKEN_KEY: &str = "secure-link-app:auth-token-key";

struct AppData {
    secure_link_client: Mutex<Option<Arc<dyn SecureLinkClient + Send + Sync>>>
}

#[tauri::command]
async fn is_running(state: State<'_, AppData>) -> Result<bool, String> {

    let maybe_client = {
        state.secure_link_client.lock().unwrap().clone()
    };

    if let Some(secure_link_client_locked) = maybe_client  {
        Ok(secure_link_client_locked.is_running().await.map_err(|e| format!("{}", e))?)
    }
    else
    {
        Ok(false)
    }

}

#[tauri::command]
async fn start(state: State<'_, AppData>) -> Result<(), String> {

    let secure_link_client = {

        let mut secure_link_client_locked = state.secure_link_client.lock().unwrap();

        match &mut *secure_link_client_locked {
            Some(secure_link_client) => secure_link_client.clone(),
            None => {

                    #[cfg(feature = "secure-link-embedded-client")]
                    let client = {
                        let auth_token = "1:RkPDgHVK85x2ycGJmpsqVoiSDMtIhS588iydbKIJqYU";
                        secure_link_embedded_client::SecureLinkEmbeddedClient::new(auth_token, "127.0.0.1", 6001)
                    };
                    #[cfg(feature = "secure-link-windows-service_manager")]
                    let client = {


                        let auth_token = "1:RkPDgHVK85x2ycGJmpsqVoiSDMtIhS588iydbKIJqYU";
                        let service_log_file_path = r"E:\source\secure_link_app\service_log.txt";

                        windows_credential_manager_rs::CredentialManager::store(SECURE_LINK_APP_AUTH_TOKEN_KEY, auth_token)
                            .expect("Failed to store token");
                        
                        secure_link_windows_service::SecureLinkWindowsService::new( 
                            "192.168.1.143", 
                            6001, 
                            auth_token,
                            service_log_file_path
                        )

                    };

                let client_arc = Arc::new(client);
                *secure_link_client_locked = Some(client_arc.clone());
                client_arc
            }
        }
    };


    match secure_link_client.start().await {
        Ok(()) => Ok(()),
        Err(SecureLinkClientError::UnauthorizedError) => Err(format!("UnauthorizedError")),
        Err(err) => Err(format!("{:?}", err)),
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            app.manage(AppData {
                secure_link_client: Mutex::new(None)
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start, 
            stop,
            is_running, 
            #[cfg(feature = "secure-link-windows-service_manager")] reinstall_service
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}