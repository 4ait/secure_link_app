
use async_trait::async_trait;
use secure_link_windows_service_manager::SecureLinkServiceError;
use windows_credential_manager_rs::CredentialManager;
use crate::SECURE_LINK_APP_AUTH_TOKEN_KEY;
use crate::secure_link_client::{SecureLinkClient, SecureLinkClientError};

pub struct SecureLinkWindowsService {
    secure_link_server_host: String,
    secure_link_server_port: u16,
    auth_token: String,
    service_log_file_path: String,
}

impl SecureLinkWindowsService {
    
    pub fn new(secure_link_server_host: &str, secure_link_server_port: u16, auth_token: &str, service_log_file_path: &str) -> Self {
        SecureLinkWindowsService {
            secure_link_server_host: secure_link_server_host.to_string(),
            secure_link_server_port,
            auth_token: auth_token.to_string(),
            service_log_file_path: service_log_file_path.to_string(),
        }
    }
    
}

#[async_trait]
impl SecureLinkClient for SecureLinkWindowsService {
    async fn start(&self) -> Result<(), SecureLinkClientError> {
        
        let start_service_result =
            secure_link_windows_service_manager::start_service(
                self.secure_link_server_host.as_str(),
                self.secure_link_server_port,
                &self.auth_token,
                &self.service_log_file_path,
            );
        
        match start_service_result { 
            
            Ok(()) => Ok(()),
            Err(error) => {
                
                match error {
                    SecureLinkServiceError::UnauthorizedError => {
                        Err(SecureLinkClientError::UnauthorizedError)
                    }
                    SecureLinkServiceError::NetworkError(err) => {
                        Err(SecureLinkClientError::NetworkError(err))
                    }
                    _ => Err(SecureLinkClientError::ServiceError(Box::new(error)))
                    
                }
                
            }
            
        }
    }

    async fn stop(&self) -> Result<(), SecureLinkClientError> {

        match secure_link_windows_service_manager::stop_service() {

            Ok(()) => Ok(()),
            Err(error) => {
                Err(SecureLinkClientError::ServiceError(Box::new(error)))
            }

        }
    }

    async fn is_running(&self) -> Result<bool, SecureLinkClientError> {

        match secure_link_windows_service_manager::is_service_running() {

            Ok(is_running) => Ok(is_running),
            Err(error) => {
                Err(SecureLinkClientError::ServiceError(Box::new(error)))
            }

        }
        
       
    }
}