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
    if data.len() < 4 {
        return None;
    }

    let mut pos = 0;
    let username_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;

    if data.len() < pos + username_len + 2 {
        return None;
    }

    let username = String::from_utf8(data[pos..pos + username_len].to_vec()).ok()?;
    pos += username_len;

    let password_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;

    if data.len() < pos + password_len {
        return None;
    }

    let password = String::from_utf8(data[pos..pos + password_len].to_vec()).ok()?;

    Some(LoginPacket { username, password })
}