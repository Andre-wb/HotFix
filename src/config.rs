use dotenvy::dotenv;
use std::env;
use std::sync::OnceLock;
pub use crate::schemas::Config;

static CONFIG: OnceLock<Config> = OnceLock::new();

impl Config {
    /// Initialize configuration from environment variables
    pub fn init() -> Result<&'static Config, String> {
        dotenv().ok();

        let config = Config {
            encryption_key: leak_string(get_env("ENCRYPTION_KEY")?),
            username_secret: leak_string(get_env("USERNAME_SECRET")?),
            session_secret: leak_string(get_env("SESSION_SECRET")?),
            database_url: leak_string(get_env("DATABASE_URL")?),
            app_environment: leak_string(get_env("APP_ENVIRONMENT")?),
            log_level: leak_string(get_env("LOG_LEVEL")?),
        };

        // Validate configuration
        config.validate()?;

        // Store in OnceLock
        CONFIG.set(config)
            .map_err(|_| "Failed to set global configuration".to_string())?;

        Ok(CONFIG.get().unwrap())
    }

    /// Get global configuration instance
    pub fn global() -> &'static Config {
        CONFIG.get().expect("Configuration not initialized. Call Config::init() first")
    }

    /// Validate configuration values
    fn validate(&self) -> Result<(), String> {
        if self.encryption_key.len() < 32 {
            return Err(format!(
                "ENCRYPTION_KEY must be at least 32 bytes, got {}",
                self.encryption_key.len()
            ));
        }

        if self.username_secret.is_empty() {
            return Err("USERNAME_SECRET cannot be empty".to_string());
        }

        if self.session_secret.is_empty() {
            return Err("SESSION_SECRET cannot be empty".to_string());
        }

        Ok(())
    }

    #[allow(unused)]
    pub fn is_development(&self) -> bool {
        self.app_environment.eq_ignore_ascii_case("development")
    }

    #[allow(unused)]
    pub fn is_production(&self) -> bool {
        self.app_environment.eq_ignore_ascii_case("production")
    }
}

/// Helper function to get environment variable
fn get_env(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("Environment variable {} not set", key))
}

/// Convert String to &'static str by leaking memory
/// Note: This is safe for configuration values that are set once and live for the entire program lifetime
fn leak_string(s: String) -> &'static str {
    Box::leak(Box::new(s))
}

#[allow(unused)]
pub fn encryption_key() -> &'static str {
    Config::global().encryption_key
}

#[allow(unused)]
pub fn username_secret() -> &'static str {
    Config::global().username_secret
}

#[allow(unused)]
pub fn session_secret() -> &'static str {
    Config::global().session_secret
}

pub fn database_url() -> &'static str {
    Config::global().database_url
}

#[allow(unused)]
pub fn app_environment() -> &'static str {
    Config::global().app_environment
}

#[allow(unused)]
pub fn log_level() -> &'static str {
    Config::global().log_level
}