//! Configuration validation
//!
//! Validates AllBeads configuration for correctness:
//! - No duplicate context names
//! - Valid URLs
//! - Required environment variables are set
//! - Paths exist where expected

use super::allbeads_config::AllBeadsConfig;
use super::boss_context::{AuthStrategy, BossContext};
use crate::AllBeadsError;
use std::collections::HashSet;

/// Validation error details
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub context: Option<String>,
    pub field: String,
    pub message: String,
}

impl ValidationError {
    fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            context: None,
            field: field.into(),
            message: message.into(),
        }
    }

    fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref ctx) = self.context {
            write!(f, "[{}] {}: {}", ctx, self.field, self.message)
        } else {
            write!(f, "{}: {}", self.field, self.message)
        }
    }
}

/// Validation result
pub type ValidationResult = std::result::Result<(), Vec<ValidationError>>;

/// Validate an AllBeads configuration
pub fn validate_config(config: &AllBeadsConfig) -> ValidationResult {
    let mut errors = Vec::new();

    // Check for contexts
    if config.contexts.is_empty() {
        errors.push(ValidationError::new(
            "contexts",
            "At least one context must be defined",
        ));
    }

    // Check for duplicate context names
    let mut seen_names = HashSet::new();
    for context in &config.contexts {
        if !seen_names.insert(&context.name) {
            errors.push(ValidationError::new(
                "contexts",
                format!("Duplicate context name: {}", context.name),
            ));
        }
    }

    // Validate each context
    for context in &config.contexts {
        if let Err(mut ctx_errors) = validate_context(context) {
            errors.append(&mut ctx_errors);
        }
    }

    // Validate agent mail port
    if config.agent_mail.port == 0 {
        errors.push(ValidationError::new(
            "agent_mail.port",
            "Port must be greater than 0",
        ));
    }

    // Validate visualization settings
    let valid_views = ["kanban", "graph", "mail", "swarm"];
    if !valid_views.contains(&config.visualization.default_view.as_str()) {
        errors.push(ValidationError::new(
            "visualization.default_view",
            format!(
                "Invalid view mode '{}'. Must be one of: {}",
                config.visualization.default_view,
                valid_views.join(", ")
            ),
        ));
    }

    let valid_themes = ["light", "dark"];
    if !valid_themes.contains(&config.visualization.theme.as_str()) {
        errors.push(ValidationError::new(
            "visualization.theme",
            format!(
                "Invalid theme '{}'. Must be one of: {}",
                config.visualization.theme,
                valid_themes.join(", ")
            ),
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a single Boss context
fn validate_context(context: &BossContext) -> ValidationResult {
    let mut errors = Vec::new();

    // Validate name
    if context.name.is_empty() {
        errors.push(
            ValidationError::new("name", "Context name cannot be empty")
                .with_context(&context.name),
        );
    }

    // Validate URL format
    if !is_valid_git_url(&context.url) {
        errors.push(
            ValidationError::new("url", format!("Invalid Git URL format: {}", context.url))
                .with_context(&context.name),
        );
    }

    // Validate authentication strategy
    if let Err(e) = validate_auth_strategy(context) {
        errors.push(e.with_context(&context.name));
    }

    // Validate environment variables
    for value in context.env_vars.values() {
        if value.starts_with('$') {
            // It's an environment variable reference
            let env_var = value.trim_start_matches('$');
            if std::env::var(env_var).is_err() {
                tracing::warn!(
                    context = %context.name,
                    env_var = %env_var,
                    "Environment variable not set (this may be intentional if set at runtime)"
                );
            }
        }
    }

    // Validate integrations
    if let Some(ref jira) = context.integrations.jira {
        if !jira.url.starts_with("http://") && !jira.url.starts_with("https://") {
            errors.push(
                ValidationError::new(
                    "integrations.jira.url",
                    format!("Invalid JIRA URL: {}", jira.url),
                )
                .with_context(&context.name),
            );
        }

        if jira.project.is_empty() {
            errors.push(
                ValidationError::new(
                    "integrations.jira.project",
                    "JIRA project key cannot be empty",
                )
                .with_context(&context.name),
            );
        }
    }

    if let Some(ref github) = context.integrations.github {
        if !github.url.starts_with("http://") && !github.url.starts_with("https://") {
            errors.push(
                ValidationError::new(
                    "integrations.github.url",
                    format!("Invalid GitHub URL: {}", github.url),
                )
                .with_context(&context.name),
            );
        }

        if github.owner.is_empty() {
            errors.push(
                ValidationError::new("integrations.github.owner", "GitHub owner cannot be empty")
                    .with_context(&context.name),
            );
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate authentication strategy requirements
fn validate_auth_strategy(context: &BossContext) -> std::result::Result<(), ValidationError> {
    match context.auth_strategy {
        AuthStrategy::GhEnterpriseToken => {
            // Check if there's an env var defined
            let has_token_env = context
                .env_vars
                .values()
                .any(|v| v.contains("TOKEN") || v.contains("GITHUB") || v.starts_with('$'));

            if !has_token_env {
                return Err(ValidationError::new(
                    "auth_strategy",
                    "gh_enterprise_token requires a GITHUB_TOKEN environment variable",
                ));
            }
        }
        AuthStrategy::PersonalAccessToken => {
            // Check if there's an env var defined
            let has_token_env = context
                .env_vars
                .values()
                .any(|v| v.contains("TOKEN") || v.starts_with('$'));

            if !has_token_env {
                return Err(ValidationError::new(
                    "auth_strategy",
                    "personal_access_token requires a token environment variable",
                ));
            }
        }
        AuthStrategy::SshAgent => {
            // SSH agent doesn't require env vars, but we could check if SSH is available
            // For now, just accept it
        }
    }

    Ok(())
}

/// Check if a string is a valid Git URL
fn is_valid_git_url(url: &str) -> bool {
    // SSH format: git@github.com:user/repo.git
    if url.starts_with("git@") && url.contains(':') {
        return true;
    }

    // HTTPS format: https://github.com/user/repo.git
    if url.starts_with("https://") || url.starts_with("http://") {
        return true;
    }

    // File path: /path/to/repo or ~/path/to/repo
    if url.starts_with('/') || url.starts_with("~/") {
        return true;
    }

    false
}

/// Validate configuration and return a Result
pub fn validate_config_result(config: &AllBeadsConfig) -> crate::Result<()> {
    validate_config(config).map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        AllBeadsError::Config(format!(
            "Configuration validation failed:\n  - {}",
            messages.join("\n  - ")
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let mut config = AllBeadsConfig::new();
        let context = BossContext::new(
            "test",
            "https://github.com/test/repo.git",
            AuthStrategy::SshAgent,
        );
        config.add_context(context);

        let result = validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_contexts() {
        let config = AllBeadsConfig::new();
        let result = validate_config(&config);
        assert!(result.is_err());

        if let Err(errors) = result {
            assert_eq!(errors.len(), 1);
            assert!(errors[0].message.contains("At least one context"));
        }
    }

    #[test]
    fn test_duplicate_context_names() {
        let mut config = AllBeadsConfig::new();

        let context1 = BossContext::new(
            "test",
            "https://github.com/test1.git",
            AuthStrategy::SshAgent,
        );

        let context2 = BossContext::new(
            "test",
            "https://github.com/test2.git",
            AuthStrategy::SshAgent,
        );

        config.add_context(context1);
        config.add_context(context2);

        let result = validate_config(&config);
        assert!(result.is_err());

        if let Err(errors) = result {
            let has_duplicate_error = errors
                .iter()
                .any(|e| e.message.contains("Duplicate context name"));
            assert!(has_duplicate_error);
        }
    }

    #[test]
    fn test_invalid_git_url() {
        let mut config = AllBeadsConfig::new();
        let context = BossContext::new("test", "not-a-valid-url", AuthStrategy::SshAgent);
        config.add_context(context);

        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_git_urls() {
        assert!(is_valid_git_url("git@github.com:user/repo.git"));
        assert!(is_valid_git_url("https://github.com/user/repo.git"));
        assert!(is_valid_git_url("http://github.com/user/repo.git"));
        assert!(is_valid_git_url("/path/to/repo"));
        assert!(is_valid_git_url("~/path/to/repo"));
        assert!(!is_valid_git_url("invalid-url"));
    }

    #[test]
    fn test_invalid_view_mode() {
        let mut config = AllBeadsConfig::new();
        config.visualization.default_view = "invalid".to_string();

        let context = BossContext::new(
            "test",
            "https://github.com/test.git",
            AuthStrategy::SshAgent,
        );
        config.add_context(context);

        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_gh_enterprise_token_validation() {
        let mut config = AllBeadsConfig::new();

        // Without env var - should fail
        let context1 = BossContext::new(
            "test1",
            "https://github.com/test.git",
            AuthStrategy::GhEnterpriseToken,
        );
        config.add_context(context1);

        let result = validate_config(&config);
        assert!(result.is_err());

        // With env var - should pass
        let mut config = AllBeadsConfig::new();
        let context2 = BossContext::new(
            "test2",
            "https://github.com/test.git",
            AuthStrategy::GhEnterpriseToken,
        )
        .with_env_var("GITHUB_TOKEN", "$MY_TOKEN");
        config.add_context(context2);

        let result = validate_config(&config);
        assert!(result.is_ok());
    }
}
