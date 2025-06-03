use crate::secure_link_client::{SecureLinkClient, SecureLinkClientError, SecureLinkClientState};
use async_trait::async_trait;
use secure_link_windows_service_manager::{SecureLinkServiceError, ServiceState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

pub struct SecureLinkWindowsService {
    secure_link_server_host: String,
    secure_link_server_port: u16,
    auth_token: String,
    service_log_file_path: String,
    // Debounce state
    last_status_query: Arc<Mutex<Option<Instant>>>,
    cached_status: Arc<Mutex<Option<SecureLinkClientState>>>,
    debounce_duration: Duration,
}

impl SecureLinkWindowsService {
    pub fn new(
        secure_link_server_host: &str,
        secure_link_server_port: u16,
        auth_token: &str,
        service_log_file_path: &str,
    ) -> Self {
        SecureLinkWindowsService {
            secure_link_server_host: secure_link_server_host.to_string(),
            secure_link_server_port,
            auth_token: auth_token.to_string(),
            service_log_file_path: service_log_file_path.to_string(),
            last_status_query: Arc::new(Mutex::new(None)),
            cached_status: Arc::new(Mutex::new(None)),
            debounce_duration: Duration::from_millis(100),
        }
    }

    pub fn with_debounce_duration(mut self, duration: Duration) -> Self {
        self.debounce_duration = duration;
        self
    }
}

#[async_trait]
impl SecureLinkClient for SecureLinkWindowsService {
    async fn start(&self) -> Result<(), SecureLinkClientError> {
        let start_service_result = secure_link_windows_service_manager::start_service(
            self.secure_link_server_host.as_str(),
            self.secure_link_server_port,
            &self.auth_token,
            &self.service_log_file_path,
        );

        // Clear cached status when starting service
        *self.cached_status.lock().await = None;
        *self.last_status_query.lock().await = None;

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
        let result = match secure_link_windows_service_manager::stop_service() {
            Ok(()) => Ok(()),
            Err(error) => Err(SecureLinkClientError::ServiceError(Box::new(error))),
        };

        // Clear cached status when stopping service
        *self.cached_status.lock().await = None;
        *self.last_status_query.lock().await = None;

        result
    }

    async fn status(&self) -> Result<SecureLinkClientState, SecureLinkClientError> {
        let now = Instant::now();
        let mut last_query = self.last_status_query.lock().await;
        let mut cached = self.cached_status.lock().await;

        // Check if we should use cached result
        if let (Some(last_time), Some(cached_state)) = (*last_query, cached.as_ref()) {
            if now.duration_since(last_time) < self.debounce_duration {
                return Ok(cached_state.clone());
            }
        }

        // Query the actual service status
        let result = match secure_link_windows_service_manager::query_state() {
            Ok(state) => match state {
                ServiceState::Running => Ok(SecureLinkClientState::Running),
                ServiceState::StartPending => Ok(SecureLinkClientState::Pending),
                ServiceState::Stopped => Ok(SecureLinkClientState::Stopped),
                _ => Ok(SecureLinkClientState::Pending),
            },
            Err(error) => Err(SecureLinkClientError::ServiceError(Box::new(error))),
        };

        // Update cache on successful query
        if let Ok(state) = &result {
            *cached = Some(state.clone());
            *last_query = Some(now);
        }

        result
    }
}