use crate::secure_link_client::{SecureLinkClient, SecureLinkClientError, SecureLinkClientState};
use async_trait::async_trait;
use log::error;
use secure_link_client::{SecureLink, SecureLinkError};
use std::sync::Arc;
use std::sync::Mutex;

pub struct SecureLinkEmbeddedClient {
    inner: Arc<SecureLinkEmbeddedClientInner>,
}

struct SecureLinkEmbeddedClientInner {
    auth_token: String,
    secure_link_server_host: String,
    secure_link_server_port: u16,
    shutdown_sender: Mutex<Option<tokio::sync::mpsc::UnboundedSender<()>>>,
    current_state: Arc<Mutex<SecureLinkClientState>>,
}

impl SecureLinkEmbeddedClient {
    pub fn new(
        auth_token: &str,
        secure_link_server_host: &str,
        secure_link_server_port: u16,
    ) -> Self {
        Self {
            inner: Arc::new(SecureLinkEmbeddedClientInner {
                auth_token: auth_token.to_string(),
                secure_link_server_host: secure_link_server_host.to_string(),
                secure_link_server_port,
                shutdown_sender: Mutex::new(None),
                current_state: Arc::new(Mutex::new(SecureLinkClientState::Stopped)),
            }),
        }
    }
}

#[async_trait]
impl SecureLinkClient for SecureLinkEmbeddedClient {
    async fn start(&self) -> Result<(), SecureLinkClientError> {
        self.inner.start().await
    }

    async fn stop(&self) -> Result<(), SecureLinkClientError> {
        self.inner.stop().await
    }

    async fn status(&self) -> Result<SecureLinkClientState, SecureLinkClientError> {
        Ok(self.inner.current_state.lock().unwrap().clone())
    }
}

impl SecureLinkEmbeddedClientInner {
    async fn start(&self) -> Result<(), SecureLinkClientError> {
        let mut shutdown_rx = {
            let current_state_ref = &mut *self.current_state.lock().unwrap();

            match current_state_ref {
                SecureLinkClientState::Running => {
                    return Ok(());
                }
                SecureLinkClientState::Pending => {
                    return Ok(());
                }
                SecureLinkClientState::Stopped => {}
            }

            *current_state_ref = SecureLinkClientState::Pending;

            let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::unbounded_channel();

            *self.shutdown_sender.lock().unwrap() = Some(shutdown_tx);

            shutdown_rx
        };

        let connect_to_global_channel_future = SecureLink::connect_to_global_channel(
            &self.secure_link_server_host,
            self.secure_link_server_port,
            &self.auth_token,
        );

        let current_state_ref_clone = { self.current_state.clone() };

        let global_channel_connect_result = tokio::select! {
            _ = shutdown_rx.recv() => {

                *current_state_ref_clone.lock().unwrap() = SecureLinkClientState::Stopped;
                return Ok(())
            }

            result = connect_to_global_channel_future => {
                result
            }

        };

        // Connect to secure link
        let secure_link = match global_channel_connect_result {
            Ok(link) => {
                *self.current_state.lock().unwrap() = SecureLinkClientState::Running;

                link
            }
            Err(err) => {
                *self.current_state.lock().unwrap() = SecureLinkClientState::Stopped;

                return match err {
                    SecureLinkError::UnauthorizedError => {
                        Err(SecureLinkClientError::UnauthorizedError)
                    }
                    _ => Err(SecureLinkClientError::NetworkError(Box::new(err))),
                };
            }
        };

        let current_state_ref_clone = self.current_state.clone();

        // Spawn the main loop
        tokio::spawn(async move {
            
            tokio::select! {
            
                 _ = shutdown_rx.recv() => {
                     // Shutdown requested
                 }
                 result = secure_link.run_message_loop() => {
            
                     if let Err(err) = result {
                           error!("Secure link main loop ended with error {err}");
                     }
            
                 }
            
             }

            *current_state_ref_clone.lock().unwrap() = SecureLinkClientState::Stopped
        });

        Ok(())
    }

    async fn stop(&self) -> Result<(), SecureLinkClientError> {
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
