//! SIWX wallet connect — EIP-1193 `personal_sign` flow (hydrate / browser only).

#[cfg(any(feature = "hydrate", test))]
fn siwx_http_error(status: u16, path: &str) -> String {
    match (status, path) {
        (401, "/auth/siwx/verify") => "Wallet signature was rejected. Try connecting again.".into(),
        (400, "/auth/siwx/verify") => {
            "Sign-in challenge expired or already used. Try again.".into()
        }
        (400, "/auth/siwx/challenge") => "Invalid wallet address or chain. Try again.".into(),
        (500, "/auth/siwx/verify") => {
            "Could not finish wallet sign-in. Try GitHub or email, or retry shortly.".into()
        }
        (502..=599, _) => "Auth service is temporarily unavailable. Try again shortly.".into(),
        _ => format!("Wallet sign-in failed ({status}). Try again."),
    }
}

/// Connect via MetaMask (or any EIP-1193 wallet) and complete SIWX sign-in.
/// Returns redirect path on success.
#[cfg(feature = "hydrate")]
pub async fn siwx_connect_evm() -> Result<String, String> {
    use gloo_net::http::Request;
    use js_sys::{Array, Object, Reflect};
    use serde::Deserialize;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    #[derive(Debug, Deserialize)]
    struct ChallengeResponse {
        nonce: String,
        message: String,
    }

    #[derive(Debug, Deserialize)]
    struct VerifyResponse {
        redirect: String,
    }

    async fn eth_request(
        ethereum: &JsValue,
        method: &str,
        params: JsValue,
    ) -> Result<JsValue, String> {
        let request = Reflect::get(ethereum, &JsValue::from_str("request"))
            .map_err(|_| "Wallet does not support request()".to_string())?;
        let request_fn: js_sys::Function = request
            .dyn_into()
            .map_err(|_| "Invalid wallet request handler".to_string())?;

        let payload = Object::new();
        Reflect::set(
            &payload,
            &JsValue::from_str("method"),
            &JsValue::from_str(method),
        )
        .map_err(|_| "Failed to build wallet request".to_string())?;
        Reflect::set(&payload, &JsValue::from_str("params"), &params)
            .map_err(|_| "Failed to build wallet request".to_string())?;

        let promise = request_fn
            .call1(ethereum, &payload)
            .map_err(|_| "Wallet request rejected".to_string())?;
        let promise: js_sys::Promise = promise
            .dyn_into()
            .map_err(|_| "Wallet returned invalid promise".to_string())?;
        JsFuture::from(promise)
            .await
            .map_err(|_| "Wallet request failed".to_string())
    }

    async fn post_json_with_credentials<T: serde::de::DeserializeOwned>(
        path: &str,
        body: serde_json::Value,
    ) -> Result<T, String> {
        let resp = Request::post(path)
            .header("Content-Type", "application/json")
            .credentials(web_sys::RequestCredentials::Include)
            .body(body.to_string())
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            return Err(siwx_http_error(resp.status(), path));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    fn parse_chain_id(chain_hex: &str) -> Result<String, String> {
        let trimmed = chain_hex.trim();
        let hex = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
            .unwrap_or(trimmed);
        let n =
            u64::from_str_radix(hex, 16).map_err(|_| "Invalid chain id from wallet".to_string())?;
        Ok(n.to_string())
    }

    let window = web_sys::window().ok_or("Browser window not available")?;
    let ethereum = Reflect::get(&window, &JsValue::from_str("ethereum"))
        .map_err(|_| "No EVM wallet found. Install MetaMask or similar.".to_string())?;
    if ethereum.is_undefined() || ethereum.is_null() {
        return Err("No EVM wallet found. Install MetaMask or similar.".to_string());
    }

    let accounts_val = eth_request(&ethereum, "eth_requestAccounts", Array::new().into()).await?;
    let accounts: Array = accounts_val
        .dyn_into()
        .map_err(|_| "No wallet account returned".to_string())?;
    let address = accounts
        .get(0)
        .as_string()
        .filter(|s| !s.is_empty())
        .ok_or("No wallet account selected".to_string())?;

    let chain_hex = eth_request(&ethereum, "eth_chainId", Array::new().into())
        .await?
        .as_string()
        .ok_or("Could not read chain id".to_string())?;
    let chain_id = parse_chain_id(&chain_hex)?;

    let challenge: ChallengeResponse = post_json_with_credentials(
        "/auth/siwx/challenge",
        serde_json::json!({
            "wallet_address": address,
            "chain_id": chain_id,
        }),
    )
    .await?;

    let sign_params = Array::new();
    sign_params.push(&JsValue::from_str(&challenge.message));
    sign_params.push(&JsValue::from_str(&address));
    let signature = eth_request(&ethereum, "personal_sign", sign_params.into())
        .await?
        .as_string()
        .ok_or("Wallet did not return a signature".to_string())?;

    let verify: VerifyResponse = post_json_with_credentials(
        "/auth/siwx/verify",
        serde_json::json!({
            "nonce": challenge.nonce,
            "signature": signature,
        }),
    )
    .await?;

    Ok(if verify.redirect.is_empty() {
        "/".to_string()
    } else {
        verify.redirect
    })
}

#[cfg(not(feature = "hydrate"))]
pub async fn siwx_connect_evm() -> Result<String, String> {
    Err("Wallet sign-in requires the app JavaScript bundle. Use GitHub or email, or run `cargo leptos build` for local wallet login.".to_string())
}

#[cfg(test)]
mod tests {
    use super::siwx_http_error;

    #[test]
    fn verify_unauthorized_is_actionable() {
        let msg = siwx_http_error(401, "/auth/siwx/verify");
        assert!(msg.contains("signature"));
    }

    #[test]
    fn verify_server_error_suggests_alternatives() {
        let msg = siwx_http_error(500, "/auth/siwx/verify");
        assert!(msg.contains("GitHub"));
    }
}
