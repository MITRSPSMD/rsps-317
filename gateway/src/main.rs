use anyhow::Result;
use tokio::net::TcpListener;
use tracing::{info, error};
use packets::parse_login_packet;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:43594";
    let listener = TcpListener::bind(addr).await?;
    
    info!("Gateway listening on {}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                info!("New connection from {}", peer_addr);
                tokio::spawn(handle_client(socket));
            }
            Err(e) => error!("Accept error: {}", e),
        }
    }
}

async fn handle_client(mut socket: tokio::net::TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    let mut buf = [0; 1024];
    match socket.read(&mut buf).await {
        Ok(n) if n > 0 => {
            if let Some(packet) = parse_login_packet(&buf[..n]) {
                info!("Parsed login packet: {:?}", packet);
                // Later: send to login service
                let _ = socket.write_all(b"Login received\n").await;
            }
        }
        Ok(_) => info!("Client disconnected"),
        Err(e) => error!("Read error: {}", e),
    }
}