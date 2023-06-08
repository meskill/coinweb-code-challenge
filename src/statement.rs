use std::time::Instant;

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

pub async fn download(server_name: ServerName) -> Result<Binary, ServerError> {
    let mut interval = time::interval(time::Duration::from_millis(100));
    println!("Start download {:?} at {:?}", server_name, Instant::now());
    for _i in 0..5 {
        interval.tick().await;

        println!("Tick download {:?} at {:?}", server_name, Instant::now());
        if rand::random::<f32>() < 0.1 {
            return Err(ServerError::Disconnected(server_name));
        }
    }
    Ok(Binary { from: server_name })
}
