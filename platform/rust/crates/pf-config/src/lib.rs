use std::env;

/// Loads configuration from environment variables with fallback defaults.
pub fn load_env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

/// Masks sensitive tokens in connection URLs and secret keys.
pub fn mask_secret(secret: &str) -> String {
    if secret.len() <= 6 {
        "***".to_string()
    } else {
        format!("{}***{}", &secret[..3], &secret[secret.len() - 3..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_helpers() {
        let port = load_env_or("NON_EXISTENT_PORT_123", 8080);
        assert_eq!(port, 8080);

        let masked = mask_secret("super_secret_password_123");
        assert_eq!(masked, "sup***123");
    }
}
