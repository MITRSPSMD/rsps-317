use anyhow::Result;
use tokio::net::TcpListener;
use tracing::{info, error};
use sqlx::postgres::PgPoolOptions;
use login::LoginService;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::Path;
use flate2::read::GzDecoder;
use std::io::Read;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = "postgresql://postgres:osrsRspsjAva317@127.0.0.1:5432/rsps_317";
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    info!("Connected to PostgreSQL");

    let login_service = Arc::new(LoginService::new(pool, "rsps-secret-key-2026".to_string()));

    let addr = "127.0.0.1:43594";
    let listener = TcpListener::bind(addr).await?;
    
    info!("Gateway listening on {}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                info!("✓ New connection from {}", peer_addr);
                let svc = login_service.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(socket, svc).await {
                        error!("Handler error: {}", e);
                    }
                });
            }
            Err(e) => error!("Accept error: {}", e),
        }
    }
}

async fn handle_client(
    mut socket: tokio::net::TcpStream,
    _login_svc: Arc<LoginService>,
) -> Result<()> {
    info!("→ Handler started");
    
    let mut buf = [0u8; 4096];
    info!("→ Waiting for data...");
    
    match socket.read(&mut buf).await {
        Ok(0) => {
            info!("→ EOF immediately");
            return Ok(());
        }
        Ok(n) => {
            info!("→ Got {} bytes", n);
            info!("→ Bytes: {:?}", &buf[0..std::cmp::min(50, n)]);
            
            if let Ok(request_str) = std::str::from_utf8(&buf[0..n]) {
                info!("→ Text: {}", request_str.trim());
                if request_str.starts_with("JAGGRAB ") {
                    info!("→ JagGrab!");
                    let path = request_str.trim_start_matches("JAGGRAB ").trim_end();
                    
                    match serve_jaggrab(path).await {
                        Ok(data) => {
                            info!("→ Sending {} bytes", data.len());
                            socket.write_all(&data).await?;
                            info!("✓ Sent");
                            return Ok(());
                        }
                        Err(e) => {
                            error!("JagGrab error: {}", e);
                            return Err(e);
                        }
                    }
                }
            }
            
            let byte = buf[0];
            info!("→ First byte: {}", byte);
            if byte == 15 {
                info!("→ OnDemandFetcher!");
                let response = vec![0u8; 8];
                socket.write_all(&response).await?;
                info!("✓ Handshake sent");
                
                loop {
                    match socket.read(&mut buf).await {
                        Ok(0) => {
                            info!("→ EOF");
                            return Ok(());
                        }
                        Ok(n) if n >= 4 => {
                            let data_type = buf[0] as usize;
                            let file_id = ((buf[1] as u16) << 8) | (buf[2] as u16);
                            info!("→ File request: type={}, id={}", data_type, file_id);
                            
                            match serve_cache_file(data_type, file_id).await {
                                Ok(data) => {
                                    socket.write_all(&data).await?;
                                    info!("✓ Sent {} bytes", data.len());
                                }
                                Err(e) => {
                                    error!("Error: {}", e);
                                    socket.write_all(&[0u8]).await?;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Read error: {}", e);
                            return Err(e.into());
                        }
                        _ => {}
                    }
                }
            }
            
            info!("→ Unknown protocol");
        }
        Err(e) => {
            error!("Initial read failed: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn serve_jaggrab(path: &str) -> Result<Vec<u8>> {
    let cache_dir = "cache";
    let dat_path = Path::new(cache_dir).join("main_file_cache.dat");
    let cache_data = tokio::fs::read(&dat_path).await?;
    
    let (idx_type, file_id) = match path {
        "/title screen" | "/title" | "/title0" => (2, 0u16),
        "/config" => (2, 1),
        "/interface" => (2, 2),
        "/media" | "/2d graphics" => (2, 3),
        "/versionlist" | "/update list" => (2, 4),
        "/textures" => (2, 5),
        "/wordenc" | "/chat system" => (2, 6),
        "/sounds" | "/sound effects" => (2, 7),
        _ => {
            info!("→ Unknown path: {}, using title0", path);
            (2, 0)
        }
    };
    
    serve_archive(idx_type, file_id, &cache_data).await
}

async fn serve_archive(cache_type: usize, file_id: u16, cache_data: &[u8]) -> Result<Vec<u8>> {
    let cache_dir = "cache";
    let idx_file = match cache_type {
        0 => "main_file_cache.idx0",
        1 => "main_file_cache.idx1",
        2 => "main_file_cache.idx2",
        3 => "main_file_cache.idx3",
        4 => "main_file_cache.idx4",
        _ => return Err(anyhow::anyhow!("Invalid cache type: {}", cache_type)),
    };
    
    let idx_path = Path::new(cache_dir).join(idx_file);
    let idx_data = tokio::fs::read(&idx_path).await?;
    
    let entry_offset = (file_id as usize) * 6;
    if entry_offset + 6 > idx_data.len() {
        return Err(anyhow::anyhow!("File {} out of range in type {}", file_id, cache_type));
    }
    
    let size_bytes = [0, idx_data[entry_offset], idx_data[entry_offset + 1], idx_data[entry_offset + 2]];
    let ptr_bytes = [0, idx_data[entry_offset + 3], idx_data[entry_offset + 4], idx_data[entry_offset + 5]];
    
    let file_size = u32::from_be_bytes(size_bytes) as usize;
    let block_ptr = u32::from_be_bytes(ptr_bytes) as usize;
    
    let data_offset = block_ptr * 512;
    let end = std::cmp::min(data_offset + file_size, cache_data.len());
    
    if data_offset >= cache_data.len() || file_size == 0 {
        return Err(anyhow::anyhow!("Invalid entry: offset={}, size={}", data_offset, file_size));
    }
    
    let compressed_data = &cache_data[data_offset..end];
    let mut decoder = GzDecoder::new(compressed_data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    
    info!("→ Decompressed type={} id={}: {} → {} bytes", cache_type, file_id, compressed_data.len(), decompressed.len());
    Ok(decompressed)
}

async fn serve_cache_file(data_type: usize, file_id: u16) -> Result<Vec<u8>> {
    let cache_dir = "cache";
    let dat_path = Path::new(cache_dir).join("main_file_cache.dat");
    let cache_data = tokio::fs::read(&dat_path).await?;
    serve_archive(data_type, file_id, &cache_data).await
}