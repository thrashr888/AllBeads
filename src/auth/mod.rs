//! Authentication module for AllBeads web app integration
//!
//! Implements AllBeads Device Code flow for CLI authentication.
//! The CLI authenticates against the AllBeads web app (allbeads.co or localhost).

use crate::config::{AllBeadsConfig, WebAuthConfig};
use crate::Result;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Check if host is localhost (for development mode)
pub fn is_localhost(host: &str) -> bool {
    host.contains("localhost") || host.contains("127.0.0.1")
}

/// Device code response from AllBeads API
#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: u64,
}

/// Access token response from AllBeads API
#[derive(Debug, Deserialize)]
pub struct AccessTokenResponse {
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub user: Option<AllBeadsUser>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_description: Option<String>,
}

/// AllBeads user from token response
#[derive(Debug, Deserialize, Clone)]
pub struct AllBeadsUser {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "githubLogin")]
    pub github_login: Option<String>,
}

/// Authentication result
#[derive(Debug, Clone, Serialize)]
pub struct AuthResult {
    pub username: String,
    pub token: String,
    pub scopes: Vec<String>,
    pub host: String,
}

/// Request a device code from AllBeads web app
pub async fn request_device_code(host: &str) -> Result<DeviceCodeResponse> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/cli/device", host);

    // Collect device info
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());
    let os = std::env::consts::OS;

    let response = client
        .post(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "host": host,
            "device_info": {
                "os": os,
                "hostname": hostname,
            }
        }))
        .send()
        .await
        .context("Failed to connect to AllBeads server")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Failed to request device code: {} - {}\n\nMake sure the AllBeads web server is running at {}",
            status,
            body,
            host
        )
        .into());
    }

    let device_code: DeviceCodeResponse = response
        .json()
        .await
        .context("Failed to parse device code response")?;

    Ok(device_code)
}

/// Poll for access token from AllBeads API
pub async fn poll_for_token(
    host: &str,
    device_code: &str,
    interval: u64,
) -> Result<AccessTokenResponse> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/cli/token", host);
    let mut poll_interval = Duration::from_secs(interval);
    let max_attempts = 180; // 15 minutes max with default 5s interval

    for attempt in 0..max_attempts {
        tokio::time::sleep(poll_interval).await;

        let response = client
            .post(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "device_code": device_code
            }))
            .send()
            .await
            .context("Failed to poll for access token")?;

        // Parse response - 400 status codes contain error info
        let token_response: AccessTokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        match token_response.error.as_deref() {
            Some("authorization_pending") => {
                // User hasn't completed authorization yet, keep polling
                tracing::debug!(attempt, "Authorization pending, continuing to poll...");
                continue;
            }
            Some("slow_down") => {
                // Increase poll interval
                poll_interval = Duration::from_secs(poll_interval.as_secs() + 5);
                tracing::debug!(
                    new_interval = poll_interval.as_secs(),
                    "Slowing down poll interval"
                );
                continue;
            }
            Some("expired_token") => {
                return Err(
                    anyhow::anyhow!("Device code expired. Please run 'ab login' again.").into(),
                );
            }
            Some("access_denied") => {
                return Err(
                    anyhow::anyhow!("Access denied. User cancelled the authorization.").into(),
                );
            }
            Some("invalid_grant") => {
                return Err(
                    anyhow::anyhow!("Invalid device code. Please run 'ab login' again.").into(),
                );
            }
            Some(error) => {
                let description = token_response.error_description.clone().unwrap_or_default();
                return Err(
                    anyhow::anyhow!("AllBeads auth error: {} - {}", error, description).into(),
                );
            }
            None => {
                // Success! We got a token
                if token_response.access_token.is_some() {
                    return Ok(token_response);
                }
            }
        }
    }

    Err(anyhow::anyhow!("Timed out waiting for authorization").into())
}

/// Validate an existing token with AllBeads API
pub async fn validate_token(host: &str, token: &str) -> Result<AllBeadsUser> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/cli/token", host);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .context("Failed to validate token")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Token validation failed: {} - {}", status, body).into());
    }

    #[derive(Deserialize)]
    struct ValidateResponse {
        #[allow(dead_code)]
        valid: bool,
        user: Option<AllBeadsUser>,
    }

    let validate_response: ValidateResponse = response
        .json()
        .await
        .context("Failed to parse validation response")?;

    validate_response
        .user
        .ok_or_else(|| anyhow::anyhow!("No user info in response").into())
}

/// Perform the complete device code authentication flow
pub async fn device_code_flow(host: &str) -> Result<AuthResult> {
    // Step 1: Request device code from AllBeads
    let device_code = request_device_code(host).await?;

    // Step 2: Display instructions to user
    let verification_url = device_code
        .verification_uri_complete
        .as_ref()
        .unwrap_or(&device_code.verification_uri);

    println!();
    println!("  Please visit: \x1b[36m{}\x1b[0m", verification_url);
    println!("  And enter code: \x1b[1m{}\x1b[0m", device_code.user_code);
    println!();
    println!("  Waiting for authorization...");

    // Try to open browser automatically
    if let Err(e) = open::that(verification_url) {
        tracing::debug!("Failed to open browser: {}", e);
    }

    // Step 3: Poll for token
    let token_response =
        poll_for_token(host, &device_code.device_code, device_code.interval).await?;

    let access_token = token_response
        .access_token
        .ok_or_else(|| anyhow::anyhow!("No access token in response"))?;

    // Extract username from user info
    let username = token_response
        .user
        .as_ref()
        .and_then(|u| u.github_login.clone().or(u.name.clone()))
        .unwrap_or_else(|| "unknown".to_string());

    Ok(AuthResult {
        username,
        token: access_token,
        scopes: vec!["cli".to_string()], // AllBeads CLI scope
        host: host.to_string(),
    })
}

/// Save authentication result to config
pub fn save_auth(config: &mut AllBeadsConfig, auth: &AuthResult) -> Result<()> {
    config.web_auth = WebAuthConfig {
        host: Some(auth.host.clone()),
        github_token: Some(auth.token.clone()), // Note: This is now an AllBeads token, not GitHub
        github_username: Some(auth.username.clone()),
        authenticated_at: Some(chrono::Utc::now().to_rfc3339()),
        scopes: auth.scopes.clone(),
    };

    config.save_default()?;
    Ok(())
}

/// Clear authentication from config
pub fn clear_auth(config: &mut AllBeadsConfig) -> Result<()> {
    config.web_auth.clear();
    config.save_default()?;
    Ok(())
}

/// Login with an existing AllBeads token (for testing/automation)
pub async fn token_login(host: &str, token: &str) -> Result<AuthResult> {
    // Validate token by calling the API
    let user = validate_token(host, token).await?;

    let username = user
        .github_login
        .or(user.name)
        .unwrap_or_else(|| "unknown".to_string());

    Ok(AuthResult {
        username,
        token: token.to_string(),
        scopes: vec!["cli".to_string()],
        host: host.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_result_serialization() {
        let result = AuthResult {
            username: "testuser".to_string(),
            token: "abs_xxx".to_string(),
            scopes: vec!["cli".to_string()],
            host: "https://allbeads.co".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_is_localhost() {
        assert!(is_localhost("http://localhost:3000"));
        assert!(is_localhost("http://127.0.0.1:3000"));
        assert!(!is_localhost("https://allbeads.co"));
    }

    #[test]
    fn test_web_auth_config_is_authenticated() {
        let config = WebAuthConfig::default();
        assert!(!config.is_authenticated());

        let config = WebAuthConfig {
            github_token: Some("token".to_string()),
            ..Default::default()
        };
        assert!(config.is_authenticated());
    }

    #[test]
    fn test_web_auth_config_host() {
        let config = WebAuthConfig::default();
        // Debug builds default to localhost, release builds to allbeads.co
        #[cfg(debug_assertions)]
        assert_eq!(config.host(), "http://localhost:3000");
        #[cfg(not(debug_assertions))]
        assert_eq!(config.host(), "https://allbeads.co");

        // Explicit host always wins
        let config = WebAuthConfig {
            host: Some("https://custom.example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(config.host(), "https://custom.example.com");
    }

    #[test]
    fn test_web_auth_config_clear() {
        let mut config = WebAuthConfig {
            host: Some("https://allbeads.co".to_string()),
            github_token: Some("token".to_string()),
            github_username: Some("user".to_string()),
            authenticated_at: Some("2024-01-01".to_string()),
            scopes: vec!["cli".to_string()],
        };

        config.clear();
        assert!(!config.is_authenticated());
        assert!(config.host.is_none());
        assert!(config.github_username.is_none());
        assert!(config.scopes.is_empty());
    }
}
