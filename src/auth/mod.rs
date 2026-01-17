//! Authentication module for AllBeads web app integration
//!
//! Implements GitHub Device Code flow for CLI authentication.
//! For localhost development, uses direct GitHub PAT authentication.

use crate::config::{AllBeadsConfig, WebAuthConfig};
use crate::Result;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// GitHub OAuth App Client ID for AllBeads CLI
/// Set via ALLBEADS_GITHUB_CLIENT_ID env var, or use default for allbeads.co
fn github_client_id() -> String {
    std::env::var("ALLBEADS_GITHUB_CLIENT_ID")
        .unwrap_or_else(|_| "Ov23liDRaWPXhE6raeLd".to_string())
}

/// Check if host is localhost (for development mode)
pub fn is_localhost(host: &str) -> bool {
    host.contains("localhost") || host.contains("127.0.0.1")
}

/// GitHub Device Code API endpoints
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const USER_API_URL: &str = "https://api.github.com/user";

/// Required OAuth scopes for AllBeads
const SCOPES: &str = "read:user,repo";

/// Device code response from GitHub
#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// Access token response from GitHub
#[derive(Debug, Deserialize)]
pub struct AccessTokenResponse {
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_description: Option<String>,
}

/// GitHub user profile
#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
    pub name: Option<String>,
    pub email: Option<String>,
}

/// Authentication result
#[derive(Debug, Clone, Serialize)]
pub struct AuthResult {
    pub username: String,
    pub token: String,
    pub scopes: Vec<String>,
    pub host: String,
}

/// Request a device code from GitHub
pub async fn request_device_code() -> Result<DeviceCodeResponse> {
    let client = reqwest::Client::new();
    let client_id = github_client_id();

    let response = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", client_id.as_str()), ("scope", SCOPES)])
        .send()
        .await
        .context("Failed to request device code from GitHub")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "GitHub device code request failed: {} - {}\n\nFor localhost development, use: ab login --with-token <your-github-token>\nCreate a token at: https://github.com/settings/tokens/new?scopes=read:user,repo",
            status,
            body
        )
        .into());
    }

    let device_code: DeviceCodeResponse = response
        .json()
        .await
        .context("Failed to parse device code response")?;

    Ok(device_code)
}

/// Poll for access token (with exponential backoff)
pub async fn poll_for_token(device_code: &str, interval: u64) -> Result<AccessTokenResponse> {
    let client = reqwest::Client::new();
    let mut poll_interval = Duration::from_secs(interval);
    let max_attempts = 60; // 5 minutes max with default 5s interval

    for attempt in 0..max_attempts {
        tokio::time::sleep(poll_interval).await;

        let client_id = github_client_id();
        let response = client
            .post(ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", client_id.as_str()),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .context("Failed to poll for access token")?;

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
            Some(error) => {
                let description = token_response.error_description.unwrap_or_default();
                return Err(
                    anyhow::anyhow!("GitHub OAuth error: {} - {}", error, description).into(),
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

/// Fetch GitHub user profile using access token
pub async fn fetch_user(token: &str) -> Result<GitHubUser> {
    let client = reqwest::Client::new();

    let response = client
        .get(USER_API_URL)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "AllBeads-CLI")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("Failed to fetch GitHub user")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("GitHub API error: {} - {}", status, body).into());
    }

    let user: GitHubUser = response
        .json()
        .await
        .context("Failed to parse GitHub user response")?;

    Ok(user)
}

/// Perform the complete device code authentication flow
pub async fn device_code_flow(host: &str) -> Result<AuthResult> {
    // Step 1: Request device code
    let device_code = request_device_code().await?;

    // Step 2: Display instructions to user
    println!();
    println!(
        "  Please visit: \x1b[36m{}\x1b[0m",
        device_code.verification_uri
    );
    println!("  And enter code: \x1b[1m{}\x1b[0m", device_code.user_code);
    println!();
    println!("  Waiting for authorization...");

    // Try to open browser automatically
    if let Err(e) = open::that(&device_code.verification_uri) {
        tracing::debug!("Failed to open browser: {}", e);
    }

    // Step 3: Poll for token
    let token_response = poll_for_token(&device_code.device_code, device_code.interval).await?;

    let access_token = token_response
        .access_token
        .ok_or_else(|| anyhow::anyhow!("No access token in response"))?;

    // Step 4: Fetch user info
    let user = fetch_user(&access_token).await?;

    let scopes = token_response
        .scope
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(AuthResult {
        username: user.login,
        token: access_token,
        scopes,
        host: host.to_string(),
    })
}

/// Save authentication result to config
pub fn save_auth(config: &mut AllBeadsConfig, auth: &AuthResult) -> Result<()> {
    config.web_auth = WebAuthConfig {
        host: Some(auth.host.clone()),
        github_token: Some(auth.token.clone()),
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

/// Login with a personal access token
pub async fn token_login(host: &str, token: &str) -> Result<AuthResult> {
    // Validate token by fetching user info
    let user = fetch_user(token).await?;

    Ok(AuthResult {
        username: user.login,
        token: token.to_string(),
        scopes: vec!["read:user".to_string(), "repo".to_string()],
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
            token: "gho_xxx".to_string(),
            scopes: vec!["repo".to_string()],
            host: "https://allbeads.co".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("testuser"));
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
        assert_eq!(config.host(), "https://allbeads.co");

        let config = WebAuthConfig {
            host: Some("http://localhost:3000".to_string()),
            ..Default::default()
        };
        assert_eq!(config.host(), "http://localhost:3000");
    }

    #[test]
    fn test_web_auth_config_clear() {
        let mut config = WebAuthConfig {
            host: Some("https://allbeads.co".to_string()),
            github_token: Some("token".to_string()),
            github_username: Some("user".to_string()),
            authenticated_at: Some("2024-01-01".to_string()),
            scopes: vec!["repo".to_string()],
        };

        config.clear();
        assert!(!config.is_authenticated());
        assert!(config.host.is_none());
        assert!(config.github_username.is_none());
        assert!(config.scopes.is_empty());
    }
}
