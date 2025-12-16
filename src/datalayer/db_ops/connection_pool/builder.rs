use std::{time::Duration};

use crate::datalayer::db_ops::constants;
use crate::datalayer::db_ops::constants::DbConfig;


/*
This is the default configuration for the database connection pool.
- max_connections: 10
- min_connections: 2
- connection_timeout: 30 seconds
- idle_timeout: 10 minutes
- max_lifetime: 30 minutes
*/
impl Default for DbConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| constants::URL.to_string()),
            max_connections: 10,
            min_connections: 2,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600), // 10 minutes
            max_lifetime: Duration::from_secs(1800), // 30 minutes
        }
    }
}

/*
This is the builder pattern for DbConfig.
- It allows for a more flexible configuration of the database connection pool.
- It also allows for a more readable configuration of the database connection pool.
- you can get the default configuration by calling DbConfig::new() and then change the values as per your requirements.
*/
// Builder pattern for DbConfig
impl DbConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_database_url(mut self, database_url: String) -> Self {
        self.database_url = database_url;
        self
    }

    pub fn set_max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = max_connections;
        self
    }

    pub fn set_min_connections(mut self, min_connections: u32) -> Self {
        self.min_connections = min_connections;
        self
    }

    pub fn set_connection_timeout(mut self, connection_timeout: Duration) -> Self {
        self.connection_timeout = connection_timeout;
        self
    }

    pub fn set_idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.idle_timeout = idle_timeout;
        self
    }

    pub fn set_max_lifetime(mut self, max_lifetime: Duration) -> Self {
        self.max_lifetime = max_lifetime;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DbConfig::default();

        // Test default values
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.max_lifetime, Duration::from_secs(1800));

        // Test that database_url is set (either from env or default)
        assert!(!config.database_url.is_empty());
    }

    #[test]
    fn test_new_config() {
        let config = DbConfig::new();

        // new() should behave the same as default()
        let default_config = DbConfig::default();
        assert_eq!(config.max_connections, default_config.max_connections);
        assert_eq!(config.min_connections, default_config.min_connections);
    }

    #[test]
    fn test_builder_set_database_url() {
        let config = DbConfig::new()
            .set_database_url("postgres://test:test@localhost:5432/testdb".to_string());

        assert_eq!(
            config.database_url,
            "postgres://test:test@localhost:5432/testdb"
        );
    }

    #[test]
    fn test_builder_set_max_connections() {
        let config = DbConfig::new().set_max_connections(20);

        assert_eq!(config.max_connections, 20);
    }

    #[test]
    fn test_builder_set_min_connections() {
        let config = DbConfig::new().set_min_connections(5);

        assert_eq!(config.min_connections, 5);
    }

    #[test]
    fn test_builder_set_connection_timeout() {
        let config = DbConfig::new().set_connection_timeout(Duration::from_secs(60));

        assert_eq!(config.connection_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_builder_set_idle_timeout() {
        let config = DbConfig::new().set_idle_timeout(Duration::from_secs(300));

        assert_eq!(config.idle_timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_builder_set_max_lifetime() {
        let config = DbConfig::new().set_max_lifetime(Duration::from_secs(3600));

        assert_eq!(config.max_lifetime, Duration::from_secs(3600));
    }

    #[test]
    fn test_builder_pattern_chaining() {
        // Test that all builder methods can be chained together
        let config = DbConfig::new()
            .set_database_url("postgres://user:pass@localhost:5432/mydb".to_string())
            .set_max_connections(50)
            .set_min_connections(10)
            .set_connection_timeout(Duration::from_secs(45))
            .set_idle_timeout(Duration::from_secs(900))
            .set_max_lifetime(Duration::from_secs(3600));
        println!("Config: {:#?}", config);
        assert_eq!(
            config.database_url,
            "postgres://user:pass@localhost:5432/mydb"
        );
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.min_connections, 10);
        assert_eq!(config.connection_timeout, Duration::from_secs(45));
        assert_eq!(config.idle_timeout, Duration::from_secs(900));
        assert_eq!(config.max_lifetime, Duration::from_secs(3600));
    }

    #[test]
    fn test_partial_builder_usage() {
        // Test that you can use only some builder methods
        let config = DbConfig::new()
            .set_max_connections(15)
            .set_database_url("postgres://custom:custom@localhost:5432/custom".to_string());
        println!("Config: {:#?}", config);
        // Custom values
        assert_eq!(config.max_connections, 15);
        assert_eq!(
            config.database_url,
            "postgres://custom:custom@localhost:5432/custom"
        );

        // Default values should remain
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_config_is_cloneable() {
        let config1 = DbConfig::new().set_max_connections(25);

        let config2 = config1.clone();
        println!("Config1: {:#?}", config1);
        println!("Config2: {:#?}", config2);
        assert_eq!(config1.max_connections, config2.max_connections);
        assert_eq!(config1.database_url, config2.database_url);
    }
}
