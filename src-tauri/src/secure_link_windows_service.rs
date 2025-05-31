use std::error::Error;
use async_trait::async_trait;
use windows_credential_manager_rs::CredentialManager;
use crate::SECURE_LINK_APP_AUTH_TOKEN_KEY;
use crate::secure_link_client::{SecureLinkClient, SecureLinkClientError};

pub struct SecureLinkWindowsService {
    secure_link_server_host: String,
    secure_link_server_port: u16
}

impl SecureLinkWindowsService {
    
    pub fn new(secure_link_server_host: &str, secure_link_server_port: u16) -> Self {
        SecureLinkWindowsService {
            secure_link_server_host: secure_link_server_host.to_string(),
            secure_link_server_port,
        }
    }
    
}

#[async_trait]
impl SecureLinkClient for SecureLinkWindowsService {
    async fn start(&self) -> Result<(), SecureLinkClientError> {

        let token = 
            CredentialManager::load(SECURE_LINK_APP_AUTH_TOKEN_KEY).expect("Failed to load token");
        
        Ok(secure_link_windows_service_manager::start_service(
            self.secure_link_server_host.as_str(),
            self.secure_link_server_port,
            &token
        )?)
    }

    async fn stop(&self) -> Result<(), Box<dyn Error>> {
        Ok(secure_link_windows_service_manager::stop_service()?)
    }

    async fn is_running(&self) -> Result<bool, Box<dyn Error>> {
        secure_link_windows_service_manager::is_service_running()
    }
}