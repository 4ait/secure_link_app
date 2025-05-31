
use async_trait::async_trait;

#[derive(thiserror::Error, Debug)]
pub enum SecureLinkClientError {
    #[error("Unauthorized")]
    UnauthorizedError,
    #[error("NetworkError")]
    NetworkError(#[from] Box<dyn std::error::Error>)
}

#[async_trait]

pub trait SecureLinkClient: Send + Sync {

    async fn start(&self) -> Result<(), SecureLinkClientError>;

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>>;

    async fn is_running(&self) -> Result<bool, Box<dyn std::error::Error>>;

}