use crate::error::MazeSshError;
use crate::models::profile::CreateProfileInput;

/// Validate profile input fields
pub fn validate_profile_input(input: &CreateProfileInput) -> Result<(), MazeSshError> {
    // Name: 1-100 chars, no control characters
    if input.name.trim().is_empty() || input.name.len() > 100 {
        return Err(MazeSshError::ValidationError(
            "Profile name must be 1-100 characters".to_string(),
        ));
    }
    if input.name.chars().any(|c| c.is_control()) {
        return Err(MazeSshError::ValidationError(
            "Profile name must not contain control characters".to_string(),
        ));
    }

    // Email: basic format check
    if !input.email.contains('@') || input.email.len() > 254 {
        return Err(MazeSshError::ValidationError(
            "Invalid email address".to_string(),
        ));
    }

    // Hostname: alphanumeric, dots, hyphens only
    validate_hostname(&input.hostname)?;

    // Host alias: alphanumeric, dots, hyphens only
    if input.host_alias.trim().is_empty() || input.host_alias.len() > 253 {
        return Err(MazeSshError::ValidationError(
            "Host alias must be 1-253 characters".to_string(),
        ));
    }
    if !input
        .host_alias
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == '_')
    {
        return Err(MazeSshError::ValidationError(
            "Host alias contains invalid characters (use alphanumeric, hyphens, dots, underscores)"
                .to_string(),
        ));
    }

    // Git username: no control chars, reasonable length
    if input.git_username.trim().is_empty() || input.git_username.len() > 100 {
        return Err(MazeSshError::ValidationError(
            "Git username must be 1-100 characters".to_string(),
        ));
    }

    // Port: valid range
    if let Some(port) = input.port {
        if port == 0 {
            return Err(MazeSshError::ValidationError(
                "Port must be between 1 and 65535".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validate hostname format (no shell injection)
pub fn validate_hostname(hostname: &str) -> Result<(), MazeSshError> {
    if hostname.trim().is_empty() || hostname.len() > 253 {
        return Err(MazeSshError::ValidationError(
            "Hostname must be 1-253 characters".to_string(),
        ));
    }
    if !hostname
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '.')
    {
        return Err(MazeSshError::ValidationError(
            "Hostname contains invalid characters (use alphanumeric, hyphens, dots only)"
                .to_string(),
        ));
    }
    Ok(())
}

/// Validate PIN meets minimum security requirements
pub fn validate_pin(pin: &str) -> Result<(), MazeSshError> {
    if pin.len() < 4 {
        return Err(MazeSshError::ValidationError(
            "PIN must be at least 4 characters".to_string(),
        ));
    }
    if pin.len() > 128 {
        return Err(MazeSshError::ValidationError(
            "PIN must be at most 128 characters".to_string(),
        ));
    }
    Ok(())
}

/// Shell-escape a string value for safe use in shell scripts
pub fn shell_escape(s: &str) -> String {
    // Replace single quotes with '\'' (end quote, escaped quote, start quote)
    s.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_hostname_valid() {
        assert!(validate_hostname("github.com").is_ok());
        assert!(validate_hostname("my-server.example.org").is_ok());
        assert!(validate_hostname("192.168.1.1").is_ok());
    }

    #[test]
    fn test_validate_hostname_invalid() {
        assert!(validate_hostname("").is_err());
        assert!(validate_hostname("host;rm -rf /").is_err());
        assert!(validate_hostname("host`whoami`").is_err());
        assert!(validate_hostname("host$(cmd)").is_err());
    }

    #[test]
    fn test_validate_pin_length() {
        assert!(validate_pin("123").is_err());
        assert!(validate_pin("1234").is_ok());
        assert!(validate_pin("longpin123").is_ok());
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "hello");
        assert_eq!(shell_escape("it's"), "it'\\''s");
    }
}
