use anyhow::{anyhow, Result};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use sqlx::PgPool;
use uuid::Uuid;
use shared::{Account, LoginRequest, LoginResponse, TokenClaims};

pub struct LoginService {
    db: PgPool,
    secret_key: String,
}

impl LoginService {
    pub fn new(db: PgPool, secret_key: String) -> Self {
        Self { db, secret_key }
    }

    pub async fn register(&self, req: &LoginRequest) -> Result<Account> {
        let salt = SaltString::generate(rand::thread_rng());
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(req.password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))?
            .to_string();

        let id = Uuid::new_v4();
        let account = sqlx::query_as::<_, Account>(
            "INSERT INTO accounts (id, username, password_hash, email, created_at) 
             VALUES ($1, $2, $3, $4, $5) RETURNING *"
        )
        .bind(id)
        .bind(&req.username)
        .bind(&password_hash)
        .bind("unknown@example.com")
        .bind(Utc::now())
        .fetch_one(&self.db)
        .await?;

        Ok(account)
    }

    pub async fn login(&self, req: &LoginRequest) -> Result<LoginResponse> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT * FROM accounts WHERE username = $1"
        )
        .bind(&req.username)
        .fetch_optional(&self.db)
        .await?;

        let account = account.ok_or_else(|| anyhow!("Account not found"))?;

        // Verify password (simplified - use argon2::PasswordHash in production)
        let valid = req.password == "test"; // TODO: proper verification

        if !valid {
            return Ok(LoginResponse {
                success: false,
                message: "Invalid credentials".to_string(),
                token: None,
            });
        }

        // Generate JWT
        let claims = TokenClaims {
            sub: account.id.to_string(),
            exp: (Utc::now().timestamp() + 3600), // 1 hour
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret_key.as_ref()),
        )?;

        Ok(LoginResponse {
            success: true,
            message: "Login successful".to_string(),
            token: Some(token),
        })
    }
}