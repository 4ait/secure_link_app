use crate::secure_link_client::{SecureLinkClient, SecureLinkClientError, SecureLinkClientState};
use async_trait::async_trait;
use secure_link_windows_service_manager::{SecureLinkServiceError, ServiceState};
pub struct SecureLinkWindowsServiceClient {
    secure_link_server_host: String,
    secure_link_server_port: u16,
    auth_token: String,
    service_log_file_path: String,
}

impl SecureLinkWindowsServiceClient {
    pub fn new(
        secure_link_server_host: &str,
        secure_link_server_port: u16,
        auth_token: &str,
        service_log_file_path: &str,
    ) -> Self {
        SecureLinkWindowsServiceClient {
            secure_link_server_host: secure_link_server_host.to_string(),
            secure_link_server_port,
            auth_token: auth_token.to_string(),
            service_log_file_path: service_log_file_path.to_string(),
        }
    }
}

#[async_trait]
impl SecureLinkClient for SecureLinkWindowsServiceClient {
    async fn start(&self) -> Result<(), SecureLinkClientError> {
        let start_service_result = secure_link_windows_service_manager::start_service(
            self.secure_link_server_host.as_str(),
            self.secure_link_server_port,
            &self.auth_token,
            &self.service_log_file_path,
        );

        match start_service_result {
            Ok(()) => Ok(()),
            Err(error) => match error {
                SecureLinkServiceError::UnauthorizedError => {
                    Err(SecureLinkClientError::UnauthorizedError)
                }
                SecureLinkServiceError::NetworkError(err) => {
                    Err(SecureLinkClientError::NetworkError(err))
                }
                _ => Err(SecureLinkClientError::ServiceError(Box::new(error))),
            },
        }
    }

    async fn stop(&self) -> Result<(), SecureLinkClientError> {
        match secure_link_windows_service_manager::stop_service() {
            Ok(()) => Ok(()),
            Err(error) => Err(SecureLinkClientError::ServiceError(Box::new(error))),
        }
    }

    async fn status(&self) -> Result<SecureLinkClientState, SecureLinkClientError> {
        match secure_link_windows_service_manager::query_state() {
            Ok(state) => match state {
                ServiceState::Running => Ok(SecureLinkClientState::Running),
                ServiceState::StartPending => Ok(SecureLinkClientState::Pending),
                ServiceState::Stopped => Ok(SecureLinkClientState::Stopped),
                _ => Ok(SecureLinkClientState::Pending),
            },
            Err(error) => Err(SecureLinkClientError::ServiceError(Box::new(error))),
        }
    }
}
