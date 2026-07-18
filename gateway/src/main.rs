use anyhow::Result;
use tokio::net::TcpListener;
use tracing::{info, error};
use sqlx::postgres::PgPoolOptions;
use login::LoginService;
use packets::parse_login_packet;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Connect to PostgreSQL
    let database_url = "postgresql://postgres:osrsRspsjAva317@127.0.0.1:5432/rsps_317";
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    info!("Connected to PostgreSQL");

    let login_service = LoginService::new(pool, "your-secret-key-here".to_string());
    let login_service = std::sync::Arc::new(login_service);

    let addr = "127.0.0.1:43594";
    let listener = TcpListener::bind(addr).await?;
    
    info!("Gateway listening on {}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                info!("New connection from {}", peer_addr);
                let svc = login_service.clone();
                tokio::spawn(handle_client(socket, svc));
            }
            Err(e) => error!("Accept error: {}", e),
        }
    }
}

async fn handle_client(
    mut socket: tokio::net::TcpStream,
    _login_svc: std::sync::Arc<login::LoginService>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    let mut buf = [0; 1024];
    match socket.read(&mut buf).await {
        Ok(n) if n > 0 => {
            if let Some(packet) = parse_login_packet(&buf[..n]) {
                info!("Parsed login packet: {:?}", packet);
                let _ = socket.write_all(b"Login received\n").await;
            }
        }
        Ok(_) => info!("Client disconnected"),
        Err(e) => error!("Read error: {}", e),
    }
}