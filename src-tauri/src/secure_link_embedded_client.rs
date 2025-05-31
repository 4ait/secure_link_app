use std::error::Error;
use std::sync::Arc;
use async_trait::async_trait;
use secure_link_client::{SecureLink, SecureLinkError};
use std::sync::Mutex;
use crate::secure_link_client::{SecureLinkClient, SecureLinkClientError};

pub struct SecureLinkEmbeddedClient {
    inner: Arc<SecureLinkEmbeddedClientInner>,
}

struct SecureLinkEmbeddedClientInner {
    auth_token: String,
    secure_link_server_host: String,
    secure_link_server_port: u16,
    shutdown_sender: Mutex<Option<tokio::sync::mpsc::UnboundedSender<()>>>,
    is_running: Arc<Mutex<bool>>,
}

impl SecureLinkEmbeddedClient {
    pub fn new(auth_token:&str, secure_link_server_host: &str, secure_link_server_port: u16) -> Self {
        Self {
            inner: Arc::new(SecureLinkEmbeddedClientInner {
                auth_token: auth_token.to_string(),
                secure_link_server_host: secure_link_server_host.to_string(),
                secure_link_server_port,
                shutdown_sender: Mutex::new(None),
                is_running: Arc::new(Mutex::new(false)),
            })
        }
    }
}

#[async_trait]
impl SecureLinkClient for SecureLinkEmbeddedClient {
    async fn start(&self) -> Result<(), SecureLinkClientError> {
        self.inner.start().await
    }

    async fn stop(&self) -> Result<(), Box<dyn Error>> {
        self.inner.stop().await
    }

    async fn is_running(&self) -> Result<bool, Box<dyn Error>> {
        Ok(*self.inner.is_running.lock().unwrap())
    }
}

impl SecureLinkEmbeddedClientInner {

    async fn start(&self) -> Result<(), SecureLinkClientError> {

        let mut shutdown_rx = {

            let is_running_ref = &mut *self.is_running.lock().unwrap();

            if *is_running_ref {
                return Ok(());
            }

            *is_running_ref = true;

            let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::unbounded_channel();

            *self.shutdown_sender.lock().unwrap() = Some(shutdown_tx);

            shutdown_rx
        };

        let connect_to_global_channel_future = SecureLink::connect_to_global_channel(
            &self.secure_link_server_host,
            self.secure_link_server_port,
            &self.auth_token
        );

        let is_running_clone = {
            self.is_running.clone()
        };
        

        let global_channel_connect_result = tokio::select! {
                _ = shutdown_rx.recv() => {
                    *is_running_clone.lock().unwrap() = false;
                    return Ok(())
                }
                result = connect_to_global_channel_future => {
                    result
                }
            };

        // Connect to secure link
        let secure_link = match global_channel_connect_result {
            Ok(link) => link,
            Err(err) => {

                return match err {
                    SecureLinkError::UnauthorizedError => {
                        Err(SecureLinkClientError::UnauthorizedError)
                    }
                    _ => {
                        Err(SecureLinkClientError::NetworkError(Box::new(err)))
                    }
                }
            }
        };

        let is_running_clone = {
            self.is_running.clone()
        };

        // Spawn the main loop

        tokio::spawn(async move {

            tokio::select! {
                _ = shutdown_rx.recv() => {
                    // Shutdown requested
                }
                _result = secure_link.run_message_loop() => {
                    // Message loop ended
                }
            }

            *is_running_clone.lock().unwrap() = false;

        });

        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn Error>> {

        // Send shutdown signal
        let sender = {
            let mut sender_guard = self.shutdown_sender.lock().unwrap();
            sender_guard.take()
        };

        if let Some(sender) = sender {
            let _ = sender.send(()); // Ignore send errors (receiver might be dropped)
        }

        Ok(())
    }
}