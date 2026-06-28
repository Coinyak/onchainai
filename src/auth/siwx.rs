//! SIWX (CAIP-122) wallet authentication — challenge + verify.

use crate::auth::session::{
    cookie_secure_for_domain, ensure_siwx_profile, issue_access_token, local_dev_host,
    post_auth_redirect_path, set_session_hint_cookie, ACCESS_TOKEN_COOKIE,
};
use crate::config::Config;
use crate::AppState;
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{Duration, Utc};
use ethers_core::types::{Address, Signature};
use ethers_core::utils::hash_message;
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature as SolSignature;
use std::str::FromStr;

const CHALLENGE_TTL_SECS: i64 = 300;

#[derive(Debug, Deserialize)]
pub struct ChallengeRequest {
    pub wallet_address: String,
    pub chain_id: String,
}

#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub nonce: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub nonce: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub ok: bool,
    pub redirect: String,
}

fn siwx_uri(config: &Config) -> String {
    if let Some(host) = local_dev_host(&config.siwx_domain) {
        format!("http://{host}:{}/auth/siwx", config.port)
    } else {
        format!("https://{}/auth/siwx", config.siwx_domain)
    }
}

fn generate_nonce() -> Result<String, StatusCode> {
    let mut bytes = [0u8; 16];
    getrandom(&mut bytes).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(B64.encode(bytes))
}

fn build_siwx_message(
    domain: &str,
    wallet_address: &str,
    chain_id: &str,
    nonce: &str,
    uri: &str,
    issued_at: chrono::DateTime<Utc>,
    expiration: chrono::DateTime<Utc>,
) -> String {
    if chain_id == "solana" {
        format!(
            "{domain} wants you to sign in with your Solana account:\n\
             {wallet_address}\n\n\
             Sign in to OnchainAI to comment, upvote, and bookmark tools\n\n\
             URI: {uri}\n\
             Version: 1\n\
             Chain ID: solana\n\
             Nonce: {nonce}\n\
             Issued At: {}\n\
             Expiration Time: {}",
            issued_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            expiration.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        )
    } else {
        format!(
            "{domain} wants you to sign in with your Ethereum account:\n\
             {wallet_address}\n\n\
             Sign in to OnchainAI to comment, upvote, and bookmark tools\n\n\
             URI: {uri}\n\
             Version: 1\n\
             Chain ID: {chain_id}\n\
             Nonce: {nonce}\n\
             Issued At: {}\n\
             Expiration Time: {}",
            issued_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            expiration.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        )
    }
}

fn normalize_wallet(wallet: &str, chain_id: &str) -> Result<String, StatusCode> {
    let w = wallet.trim();
    if w.is_empty() || w.len() > 128 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if chain_id == "solana" {
        Pubkey::from_str(w).map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(w.to_string())
    } else {
        let addr: Address = w.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(format!("{addr:#x}"))
    }
}

fn verify_evm_signature(message: &str, signature_hex: &str, wallet: &str) -> bool {
    let Ok(sig) = signature_hex.parse::<Signature>() else {
        return false;
    };
    let Ok(addr) = wallet.parse::<Address>() else {
        return false;
    };
    sig.verify(hash_message(message), addr).is_ok()
}

fn verify_solana_signature(message: &str, signature_b58: &str, wallet: &str) -> bool {
    let Ok(pubkey) = wallet.parse::<Pubkey>() else {
        return false;
    };
    let Ok(sig) = signature_b58.parse::<SolSignature>() else {
        return false;
    };
    sig.verify(pubkey.as_ref(), message.as_bytes())
}

fn set_session_cookie(name: &str, value: &str, max_age_secs: i64, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    // SameSite=Strict for CSRF hardening (SECURITY.md). SIWX verify is a
    // same-site fetch, so the session cookie is still sent on later requests.
    format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age_secs}{secure_flag}"
    )
}

/// `POST /auth/siwx/challenge` — server-generated CAIP-122 message + nonce.
pub async fn challenge(
    State(state): State<AppState>,
    Json(body): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    let config = &state.config;
    let wallet = normalize_wallet(&body.wallet_address, &body.chain_id)?;
    let nonce = generate_nonce()?;
    let issued_at = Utc::now();
    let expiration = issued_at + Duration::seconds(CHALLENGE_TTL_SECS);
    let uri = siwx_uri(config);
    let message = build_siwx_message(
        &config.siwx_domain,
        &wallet,
        &body.chain_id,
        &nonce,
        &uri,
        issued_at,
        expiration,
    );

    sqlx::query(
        r#"
        INSERT INTO siwx_sessions (
            nonce, wallet_address, chain_id, message, signature,
            issued_at, expiration_time, used
        )
        VALUES ($1, $2, $3, $4, '', $5, $6, false)
        "#,
    )
    .bind(&nonce)
    .bind(&wallet)
    .bind(&body.chain_id)
    .bind(&message)
    .bind(issued_at)
    .bind(expiration)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ChallengeResponse { nonce, message }))
}

/// `POST /auth/siwx/verify` — validate signature, upsert profile, set session cookie.
pub async fn verify(
    State(state): State<AppState>,
    Json(body): Json<VerifyRequest>,
) -> Result<Response, StatusCode> {
    let config = &state.config;

    // Consume the nonce under a row lock so two concurrent verifications of the
    // same challenge cannot both succeed. The first transaction to grab the
    // `FOR UPDATE` lock marks `used = true` and commits; any concurrent waiter
    // then reads `used = true` and is rejected, preventing signature replay.
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = sqlx::query_as::<_, SiwxRow>(
        r#"
        SELECT nonce, wallet_address, chain_id, message, expiration_time, used
        FROM siwx_sessions
        WHERE nonce = $1
        FOR UPDATE
        "#,
    )
    .bind(&body.nonce)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::BAD_REQUEST)?;

    if row.used {
        return Err(StatusCode::BAD_REQUEST);
    }
    if row.expiration_time < Utc::now() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let sig_ok = if row.chain_id == "solana" {
        verify_solana_signature(&row.message, &body.signature, &row.wallet_address)
    } else {
        verify_evm_signature(&row.message, &body.signature, &row.wallet_address)
    };
    if !sig_ok {
        // Dropping `tx` rolls back the lock without consuming the nonce, so a
        // genuine retry with a correct signature can still succeed.
        return Err(StatusCode::UNAUTHORIZED);
    }

    let user_id = match ensure_siwx_profile(&state.pool, config, &row.wallet_address, &row.chain_id)
        .await
    {
        Ok(id) => id,
        Err(err) => {
            tracing::error!(error = %err, wallet = %row.wallet_address, "SIWX profile setup failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    sqlx::query(
        r#"
        UPDATE siwx_sessions
        SET signature = $1, used = true, used_at = now(), profile_id = $2
        WHERE nonce = $3
        "#,
    )
    .bind(&body.signature)
    .bind(user_id)
    .bind(&body.nonce)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let token = issue_access_token(
        user_id,
        &config.jwt_secret,
        config.siwx_session_ttl,
        &config.jwt_issuer(),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let secure_cookie = cookie_secure_for_domain(&config.siwx_domain);
    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        set_session_cookie(
            ACCESS_TOKEN_COOKIE,
            &token,
            config.siwx_session_ttl,
            secure_cookie,
        )
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    headers.append(
        header::SET_COOKIE,
        set_session_hint_cookie(config.siwx_session_ttl, secure_cookie)
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    let redirect = post_auth_redirect_path(&state.pool, user_id).await;
    Ok((headers, Json(VerifyResponse { ok: true, redirect })).into_response())
}

#[derive(Debug, sqlx::FromRow)]
struct SiwxRow {
    #[allow(dead_code)]
    nonce: String,
    wallet_address: String,
    chain_id: String,
    message: String,
    expiration_time: chrono::DateTime<Utc>,
    used: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn siwx_message_includes_domain_and_nonce() {
        let issued = Utc::now();
        let exp = issued + Duration::seconds(300);
        let msg = build_siwx_message(
            "www.onchain-ai.xyz",
            "0x1234567890abcdef1234567890abcdef12345678",
            "1",
            "abc123",
            "https://www.onchain-ai.xyz/auth/siwx",
            issued,
            exp,
        );
        assert!(msg.contains("www.onchain-ai.xyz wants you to sign in"));
        assert!(msg.contains("Nonce: abc123"));
        assert!(msg.contains("Chain ID: 1"));
    }

    #[test]
    fn session_cookie_uses_strict_samesite() {
        let cookie = set_session_cookie(ACCESS_TOKEN_COOKIE, "tok", 86_400, true);
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("; Secure"));
    }

    #[test]
    fn solana_message_uses_solana_chain_id() {
        let issued = Utc::now();
        let exp = issued + Duration::seconds(300);
        let msg = build_siwx_message(
            "www.onchain-ai.xyz",
            "So11111111111111111111111111111111111111112",
            "solana",
            "n1",
            "https://www.onchain-ai.xyz/auth/siwx",
            issued,
            exp,
        );
        assert!(msg.contains("Solana account"));
        assert!(msg.contains("Chain ID: solana"));
    }
}
