use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginPacket {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
}

pub fn parse_login_packet(data: &[u8]) -> Option<LoginPacket> {
    // For now, just a placeholder
    // Later: implement actual 317 binary protocol parsing
    if data.len() < 2 {
        return None;
    }
    
    Some(LoginPacket {
        username: "test_user".to_string(),
        password: "test_pass".to_string(),
    })
}