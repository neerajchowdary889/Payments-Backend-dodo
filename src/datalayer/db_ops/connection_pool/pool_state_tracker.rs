use crate::datalayer::db_ops::constants::types::{DbConfig, PoolStateTracker};
use sqlx::Postgres;
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgPoolOptions;
use tracing::{error, info};

/*
This is the pool state tracker.
- It is used to track the state of the connection pool.
- It is used to monitor the number of connections in the pool.
- It is used to monitor the number of connections that are currently in use.

It will Eager load all the connections while starting the application and keep them in the memory (struct).
- Reason for this is to avoid the overhead of creating and destroying connections every time a request is made.
- This will also help to mitigate the errors occurring while running the application.
  If any errors, then those would be occurred while application starting so that this can be debugged easily.
- Better Reusability of the connections.
*/

impl PoolStateTracker {
    /// Creates a new pool state tracker (non-singleton version)
    /// For singleton access, use `get_or_init_global` instead
    pub fn new(db_config: DbConfig) -> Self {
        let tracker = Self {
            db_config,
            current_connections: Vec::new(),
        };
        tracker.eager_load().await.unwrap();
        tracker
    }

    /// Gets or initializes the global singleton pool state tracker
    /// This is thread-safe and ensures only one instance is created
    pub fn get_or_init_global(
        db_config: Option<DbConfig>,
    ) -> Result<&'static PoolStateTracker, sqlx::Error> {
        use crate::datalayer::db_ops::constants::constants::POOL_STATE_TRACKER;

        if db_config.is_none() {
            // Check if already initialized
            if let Some(tracker) = POOL_STATE_TRACKER.get() {
                return Ok(tracker);
            }
            error!("Database configuration is not initialized and no existing tracker found");
            return Err(sqlx::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Database configuration is not initialized",
            )));
        }

        // Try to initialize or get existing
        POOL_STATE_TRACKER.get_or_init(|| {
            info!("Initializing global pool state tracker");
            Self::new(db_config.unwrap())
        });

        Ok(POOL_STATE_TRACKER.get().unwrap())
    }

    /// Returns the current number of tracked connections
    pub fn connection_count(&self) -> usize {
        self.current_connections.len()
    }

    /// This will fill all the connections in the pool using eager loading.
    /// Creates a temporary pool and acquires all connections upfront.
    pub async fn eager_load(&mut self) -> Result<(), sqlx::Error> {
        info!(
            "Eager loading {} connections...",
            self.db_config.max_connections
        );

        // Create a temporary pool to acquire connections from
        let pool = PgPoolOptions::new()
            .max_connections(self.db_config.max_connections)
            .min_connections(self.db_config.min_connections)
            .acquire_timeout(self.db_config.connection_timeout)
            .idle_timeout(self.db_config.idle_timeout)
            .max_lifetime(self.db_config.max_lifetime)
            .connect(&self.db_config.database_url)
            .await
            .map_err(|e| {
                error!("Failed to create pool for eager loading: {}", e);
                e
            })?;

        // Acquire connections up to max_connections
        for i in 0..self.db_config.max_connections {
            match pool.acquire().await {
                Ok(conn) => {
                    info!(
                        "Acquired connection {}/{}",
                        i + 1,
                        self.db_config.max_connections
                    );
                    self.add_connection(conn);
                }
                Err(e) => {
                    error!("Failed to acquire connection {}: {}", i + 1, e);
                    return Err(e);
                }
            }
        }

        info!(
            "Successfully eager loaded {} connections",
            self.connection_count()
        );
        Ok(())
    }

    /// Adds a connection to track
    pub fn add_connection(&mut self, conn: PoolConnection<Postgres>) {
        self.current_connections.push(conn);
    }

    /// Removes and returns a connection if available
    pub fn remove_connection(&mut self) -> Option<PoolConnection<Postgres>> {
        self.current_connections.pop()
    }

    /// Clears all tracked connections
    /// Note: This will drop all connections, returning them to the pool
    pub fn clear(&mut self) {
        info!("Clearing {} tracked connections", self.connection_count());
        self.current_connections.clear();
    }

    /// Returns true if the tracker has no connections
    pub fn is_empty(&self) -> bool {
        self.current_connections.is_empty()
    }

    /// Returns the maximum allowed connections from config
    pub fn max_connections(&self) -> u32 {
        self.db_config.max_connections
    }
}
