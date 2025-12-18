use crate::datalayer::db_ops::constants::constants::POOL_STATE_TRACKER;
use crate::datalayer::db_ops::constants::types::{DbConfig, PoolStateTracker};
use sqlx::Postgres;
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgPoolOptions;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::{error, info, instrument};

/*
This is the pool state tracker.
- It is used to track the state of the connection pool.
- It is used to monitor the number of connections in the pool.
- It is used to monitor the number of connections that are currently in use.

It will Eager load the minimum required connections while starting the application and keep them in the memory (struct).
- Reason for this is to avoid the overhead of creating and destroying connections every time a request is made.
- This will also help to mitigate the errors occurring while running the application.
  If any errors, then those would be occurred while application starting so that this can be debugged easily.
- If all connections are created at once, that lead to high load on the database, higher startup times when there are more connections and can cause performance issues.
  so we are eager loading only the minimum required connections.
  Remaining connections are pulled on demand.
*/
impl PoolStateTracker {
    /// Creates a new pool state tracker (non-singleton version)
    /// For singleton access, use `new()` instead
    #[instrument(fields(service = "PoolStateTracker"))]
    pub async fn init(db_config: DbConfig) -> Result<Self, sqlx::Error> {
        info!("Creating pool state tracker with hybrid loading strategy");

        // Create the pool first
        let pool = PgPoolOptions::new()
            .max_connections(db_config.max_connections)
            .min_connections(db_config.min_connections)
            .acquire_timeout(db_config.connection_timeout)
            .idle_timeout(db_config.idle_timeout)
            .max_lifetime(db_config.max_lifetime)
            .connect(&db_config.database_url)
            .await
            .map_err(|e| {
                error!("Failed to create pool: {}", e);
                e
            })?;

        let mut tracker = Self {
            current_connections: Mutex::new(Vec::new()),
            available_connections: AtomicU32::new(db_config.max_connections.clone()),
            db_config: db_config,
            pool: std::sync::Arc::new(pool),
        };

        // Eager load only min_connections
        tracker.eager_load().await?;
        Ok(tracker)
    }

    /// Creates or returns the global singleton pool state tracker
    /// This is thread-safe and ensures only one instance is created
    #[instrument(fields(service = "PoolStateTracker"))]
    pub async fn new(
        db_config: Option<DbConfig>,
    ) -> Result<&'static PoolStateTracker, sqlx::Error> {
        // Check if already initialized
        // Idempotent function
        if let Some(tracker) = POOL_STATE_TRACKER.get() {
            return Ok(tracker);
        }

        // If no db_config is provided, return error
        if db_config.is_none() {
            error!("Database configuration is not initialized and no existing tracker found");
            return Err(sqlx::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Database configuration is not initialized",
            )));
        }

        // Initialize the tracker
        info!("Initializing global pool state tracker");
        let tracker = Self::init(db_config.unwrap()).await?;

        // Store in OnceLock (handle race condition)
        match POOL_STATE_TRACKER.set(tracker) {
            Ok(_) => Ok(POOL_STATE_TRACKER.get().unwrap()),
            Err(_) => {
                // Another thread won the race, use their instance
                Ok(POOL_STATE_TRACKER.get().unwrap())
            }
        }
    }

    /// Eager loads only the minimum required connections (min_connections)
    /// Remaining connections are loaded lazily on-demand
    #[instrument(fields(service = "PoolStateTracker"))]
    pub async fn eager_load(&self) -> Result<(), sqlx::Error> {
        info!(
            "Eager loading {} core connections (min_connections)...",
            self.db_config.min_connections
        );

        // Acquire only min_connections upfront
        for i in 0..self.db_config.min_connections {
            match self.pool.acquire().await {
                Ok(conn) => {
                    info!(
                        "Acquired core connection {}/{}",
                        i + 1,
                        self.db_config.min_connections
                    );
                    self.add_connection(conn);
                }
                Err(e) => {
                    error!("Failed to acquire core connection {}: {}", i + 1, e);
                    return Err(e);
                }
            }
        }

        info!(
            "Successfully eager loaded {} core connections",
            self.connection_count()
        );
        Ok(())
    }

    /// Acquires a connection on-demand if pool has capacity
    /// Returns cached connection if available, otherwise acquires from pool
    /// Atomically decrements the available connection count
    #[instrument(fields(service = "PoolStateTracker"))]
    pub async fn get_connection(&self) -> Result<PoolConnection<Postgres>, sqlx::Error> {
        // First, try to use cached connection
        if let Some(conn) = self.remove_connection() {
            info!("Reusing cached connection");
            // Decrement available connections atomically
            let prev = self.available_connections.fetch_sub(1, Ordering::SeqCst);
            info!("Available connections: {} -> {}", prev, prev - 1);
            return Ok(conn);
        }

        // If no cached connections, acquire from pool
        info!(
            "Lazy loading connection (current: {}, max: {})",
            self.connection_count(),
            self.db_config.max_connections
        );
        let conn = self.pool.acquire().await.map_err(|e| {
            error!("Failed to lazy load connection: {}", e);
            e
        })?;

        // Decrement available connections atomically
        let prev = self.available_connections.fetch_sub(1, Ordering::SeqCst);
        info!("Available connections: {} -> {}", prev, prev - 1);
        Ok(conn)
    }

    /// Returns a connection to the cache if there's capacity
    /// Atomically increments the available connection count
    /// Cache only holds min_connections (core/hot connections)
    /// Lazy-loaded connections are dropped when returned
    #[instrument(fields(service = "PoolStateTracker"))]
    pub fn return_connection(&self, conn: PoolConnection<Postgres>) {
        // Increment available connections atomically
        let prev = self.available_connections.fetch_add(1, Ordering::SeqCst);
        info!("Available connections: {} -> {}", prev, prev + 1);

        // Only cache up to min_connections (core connections)
        // Lazy-loaded connections beyond min are dropped
        if (self.connection_count() as u32) < self.db_config.min_connections {
            info!(
                "Caching connection for reuse (cache: {}/{})",
                self.connection_count() + 1,
                self.db_config.min_connections
            );
            self.add_connection(conn);
        } else {
            info!("Cache full or lazy connection, dropping connection back to pool");
            drop(conn);
        }
    }

    /// Returns the current number of tracked connections
    fn connection_count(&self) -> usize {
        self.current_connections.lock().unwrap().len()
    }

    /// Returns the current number of available connections (atomic read)
    pub fn available_connections(&self) -> u32 {
        self.available_connections.load(Ordering::SeqCst)
    }
    /// Adds a connection to track
    pub fn add_connection(&self, conn: PoolConnection<Postgres>) {
        self.current_connections.lock().unwrap().push(conn);
    }

    /// Removes and returns a connection if available
    pub fn remove_connection(&self) -> Option<PoolConnection<Postgres>> {
        self.current_connections.lock().unwrap().pop()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datalayer::db_ops::constants::DbConfig;

    #[tokio::test]
    async fn test_hybrid_loading_eager_only_min_connections() {
        use crate::datalayer::db_ops::constants;
        println!("\n=== TEST: Hybrid Loading - Eager Only Min Connections ===");

        // Skip if DATABASE_URL is not set
        if std::env::var("DATABASE_URL").is_err() && constants::URL == "" {
            println!("⚠️  Skipping test: DATABASE_URL not set");
            return;
        }

        println!("📋 Creating config: min=2, max=5");
        let config = DbConfig::default()
            .set_min_connections(2)
            .set_max_connections(5);

        println!("🔧 Initializing PoolStateTracker...");
        let tracker = PoolStateTracker::init(config).await.unwrap();

        println!("📊 Tracker initialized:");
        println!("   - Cached connections: {}", tracker.connection_count());
        println!(
            "   - Available connections: {}",
            tracker.available_connections()
        );
        println!("   - Max connections: {}", tracker.max_connections());

        // Should have eagerly loaded only min_connections
        assert_eq!(tracker.connection_count(), 2);
        println!("✅ Test passed: Eagerly loaded exactly min_connections (2)");
    }

    #[tokio::test]
    async fn test_lazy_loading_on_demand() {
        println!("\n=== TEST: Lazy Loading On Demand ===");

        if std::env::var("DATABASE_URL").is_err() {
            println!("⚠️  Skipping test: DATABASE_URL not set");
            return;
        }

        println!("📋 Creating config: min=1, max=3");
        let config = DbConfig::default()
            .set_min_connections(1)
            .set_max_connections(3);

        println!("🔧 Initializing PoolStateTracker...");
        let mut tracker = PoolStateTracker::init(config).await.unwrap();

        println!("📊 Initial state:");
        println!("   - Cached connections: {}", tracker.connection_count());
        println!(
            "   - Available connections: {}",
            tracker.available_connections()
        );
        assert_eq!(tracker.connection_count(), 1);

        // Lazy load additional connection
        println!("\n🔄 Getting connection from tracker...");
        let conn = tracker.get_connection().await.unwrap();

        println!("📊 After get_connection:");
        println!("   - Cached connections: {}", tracker.connection_count());
        println!(
            "   - Available connections: {}",
            tracker.available_connections()
        );

        // Connection was acquired (not from cache since cache is now empty)
        assert_eq!(tracker.connection_count(), 0);
        println!("✅ Cache is now empty (connection was taken)");

        // Return connection to cache
        println!("\n↩️  Returning connection to tracker...");
        tracker.return_connection(conn);

        println!("📊 After return_connection:");
        println!("   - Cached connections: {}", tracker.connection_count());
        println!(
            "   - Available connections: {}",
            tracker.available_connections()
        );

        // Should now have cached the connection
        assert_eq!(tracker.connection_count(), 1);
        println!("✅ Test passed: Connection successfully returned to cache");
    }

    #[tokio::test]
    async fn test_connection_reuse_from_cache() {
        println!("\n=== TEST: Connection Reuse From Cache ===");

        if std::env::var("DATABASE_URL").is_err() {
            println!("⚠️  Skipping test: DATABASE_URL not set");
            return;
        }

        println!("📋 Creating config: min=2, max=5");
        let config = DbConfig::default()
            .set_min_connections(2)
            .set_max_connections(5);

        println!("🔧 Initializing PoolStateTracker...");
        let mut tracker = PoolStateTracker::init(config).await.unwrap();

        println!("📊 Initial state:");
        println!("   - Cached connections: {}", tracker.connection_count());
        println!(
            "   - Available connections: {}",
            tracker.available_connections()
        );
        assert_eq!(tracker.connection_count(), 2);

        // Get connection from cache
        println!("\n🔄 Getting connection from cache...");
        let conn1 = tracker.get_connection().await.unwrap();

        println!("📊 After get_connection:");
        println!("   - Cached connections: {}", tracker.connection_count());
        println!(
            "   - Available connections: {}",
            tracker.available_connections()
        );
        assert_eq!(tracker.connection_count(), 1); // One removed from cache
        println!("✅ One connection removed from cache");

        // Return it
        println!("\n↩️  Returning connection back to cache...");
        tracker.return_connection(conn1);

        println!("📊 After return_connection:");
        println!("   - Cached connections: {}", tracker.connection_count());
        println!(
            "   - Available connections: {}",
            tracker.available_connections()
        );
        assert_eq!(tracker.connection_count(), 2); // Back in cache
        println!("✅ Test passed: Connection successfully reused from cache");
    }

    #[tokio::test]
    async fn test_max_connections_respected() {
        println!("\n=== TEST: Max Connections Respected ===");

        if std::env::var("DATABASE_URL").is_err() {
            println!("⚠️  Skipping test: DATABASE_URL not set");
            return;
        }

        println!("📋 Creating config: min=1, max=2");
        let config = DbConfig::default()
            .set_min_connections(1)
            .set_max_connections(2);

        println!("🔧 Initializing PoolStateTracker...");
        let tracker = PoolStateTracker::init(config).await.unwrap();

        println!("📊 Tracker state:");
        println!("   - Cached connections: {}", tracker.connection_count());
        println!(
            "   - Available connections: {}",
            tracker.available_connections()
        );
        println!("   - Max connections: {}", tracker.max_connections());

        // Verify max_connections is accessible
        assert_eq!(tracker.max_connections(), 2);
        println!("✅ Test passed: Max connections correctly set to 2");
    }
}
