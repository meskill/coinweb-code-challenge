use std::time::Instant;

use async_trait::async_trait;
use derive_more::Display;
use thiserror::Error;
use tokio::time;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Server {0:?}: abruptly disconnected")]
    Disconnected(ServerName),
}

#[derive(Display, Debug)]
#[display(fmt = "Binary[source='{}']", "from.0")]
pub struct Binary {
    #[allow(dead_code)]
    from: ServerName,
}

#[derive(Debug, Clone)]
pub struct ServerName(pub String);

#[async_trait]
pub trait Download<T, E> {
    async fn download(self) -> Result<T, E>;
}

#[async_trait]
impl Download<Binary, ServerError> for ServerName {
    async fn download(self) -> Result<Binary, ServerError> {
        let mut interval = time::interval(time::Duration::from_millis(100));
        println!("Start download {:?} at {:?}", self, Instant::now());
        for _i in 0..5 {
            interval.tick().await;

            println!("Tick download {:?} at {:?}", self, Instant::now());
            if rand::random::<f32>() < 0.1 {
                return Err(ServerError::Disconnected(self));
            }
        }
        Ok(Binary { from: self })
    }
}
