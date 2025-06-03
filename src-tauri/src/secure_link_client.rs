use async_trait::async_trait;

#[derive(thiserror::Error, Debug)]
pub enum SecureLinkClientError {
    #[error("Unauthorized")]
    UnauthorizedError,

    #[cfg(feature = "secure-link-windows-service_manager")]
    #[error("ServiceError")]
    ServiceError(Box<dyn std::error::Error>),

    #[error("NetworkError")]
    NetworkError(Box<dyn std::error::Error>),
}

#[derive(Debug, Clone)]
pub enum SecureLinkClientState {
    Running,
    Pending,
    Stopped,
}

#[async_trait]

pub trait SecureLinkClient: Send + Sync {
    async fn start(&self) -> Result<(), SecureLinkClientError>;

    async fn stop(&self) -> Result<(), SecureLinkClientError>;

    async fn status(&self) -> Result<SecureLinkClientState, SecureLinkClientError>;
}
