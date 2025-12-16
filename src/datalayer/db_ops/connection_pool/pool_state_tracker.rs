// Re-export PoolStateTracker from constants module
pub use crate::datalayer::db_ops::constants::types::{PoolStateTracker, DbConfig};


/*
This is the pool state tracker.
- It is used to track the state of the connection pool.
- It is used to monitor the number of connections in the pool.
- It is used to monitor the number of connections that are currently in use.

It will Eager load all the connections while starting the application and keep them in the memory (struct).
- Reason for this is to avoid the overhead of creating and destroying connections every time a request is made.
- This will also help to mitigate the errors occurring while running the application. 
  If any errors, then those would be occured while appliation starting so tht this can be debugged easily.
- Better Reusability of the connections.
*/

impl PoolStateTracker {
    /// Creates a new pool state tracker
    pub fn new(db_config: DbConfig) -> Self {
        Self {
            db_config,
            current_connections: Vec::new(),
            current_connections_count: 0,
        }
    }

    /// Returns the current number of tracked connections
    pub fn connection_count(&self) -> usize {
        self.current_connections.len()
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
    pub fn clear(&mut self) {
        self.current_connections.clear();
    }
}
