use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use crate::secure_link_client::{SecureLinkClient, SecureLinkClientError};
use crate::secure_link_embedded_client::SecureLinkEmbeddedClient;

mod secure_link_client;
#[cfg(feature = "secure-link-windows-service_manager")]
mod secure_link_windows_service;

#[cfg(feature = "secure-link-embedded-client")]
mod secure_link_embedded_client;


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

                let auth_token = "1:5hoZe5BoaCnfMkpbV_aVDyJfmfXreviP-4Jx9PDjByA";
                let client = SecureLinkEmbeddedClient::new(auth_token, "127.0.0.1", 6001);
                let client_arc = Arc::new(client);
                *secure_link_client_locked = Some(client_arc.clone());
                client_arc
            }
        }
    };


    match secure_link_client.start().await {
        Ok(()) => Ok(()),
        Err(SecureLinkClientError::UnauthorizedError) => Err(format!("UnauthorizedError")),
        Err(err) => Err(format!("{}", err)),
    }

}

#[tauri::command]
async fn stop(state: State<'_, AppData>) -> Result<(), String> {

    let maybe_client_clone = {
        state.secure_link_client.lock().unwrap().as_ref().map(|client| { client.clone() })
    };

    if let Some(secure_link_client) = maybe_client_clone {
        secure_link_client.stop().await.map_err(|e| format!("{}", e))?;
    }

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
        .invoke_handler(tauri::generate_handler![start, stop, is_running])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}