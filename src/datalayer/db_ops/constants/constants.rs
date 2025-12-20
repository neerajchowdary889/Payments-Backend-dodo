use crate::datalayer::db_ops::constants::types::PoolStateTracker;
use std::sync::OnceLock;

pub const URL: &str = "postgres://postgres:postgres@localhost:5455/payments_db";
pub const DENOMINATOR: i64 = 10000;

// Thread-safe singleton pattern for pool state tracker
// OnceLock ensures the value is initialized only once and is safe to access from multiple threads
pub static POOL_STATE_TRACKER: OnceLock<PoolStateTracker> = OnceLock::new();
