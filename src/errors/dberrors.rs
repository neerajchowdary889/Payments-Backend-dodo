use std::fmt;

#[derive(Debug)]
pub enum DbError {
    ConnectionError,
    QueryError,
    TransactionError,
    PoolError,
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::ConnectionError => write!(f, "Database connection error"),
            DbError::QueryError => write!(f, "Database query error"),
            DbError::TransactionError => write!(f, "Database transaction error"),
            DbError::PoolError => write!(f, "Database pool error"),
        }
    }
}

impl std::error::Error for DbError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_error_display() {
        let error = DbError::ConnectionError;
        println!("{}", error);
        assert_eq!(format!("{}", error), "Database connection error");
    }

    #[test]
    fn test_query_error_display() {
        let error = DbError::QueryError;
        println!("{}", error);
        assert_eq!(format!("{}", error), "Database query error");
    }

    #[test]
    fn test_transaction_error_display() {
        let error = DbError::TransactionError;
        println!("{}", error);
        assert_eq!(format!("{}", error), "Database transaction error");
    }

    #[test]
    fn test_pool_error_display() {
        let error = DbError::PoolError;
        println!("{}", error);
        assert_eq!(format!("{}", error), "Database pool error");
    }

    #[test]
    fn test_to_string_conversion() {
        let error = DbError::ConnectionError;
        let error_string = error.to_string();
        println!("{}", error_string);
        assert_eq!(error_string, "Database connection error");
    }

    #[test]
    fn test_all_variants_to_string() {
        let errors = vec![
            (DbError::ConnectionError, "Database connection error"),
            (DbError::QueryError, "Database query error"),
            (DbError::TransactionError, "Database transaction error"),
            (DbError::PoolError, "Database pool error"),
        ];

        for (error, expected_msg) in errors {
            assert_eq!(error.to_string(), expected_msg);
        }
    }

    #[test]
    fn test_debug_format() {
        let error = DbError::ConnectionError;
        let debug_output = format!("{:?}", error);
        println!("{}", debug_output);
        assert_eq!(debug_output, "ConnectionError");
    }

    #[test]
    fn test_error_trait_implementation() {
        // Test that DbError implements std::error::Error
        let error: Box<dyn std::error::Error> = Box::new(DbError::QueryError);
        println!("{}", error.to_string());
        assert_eq!(error.to_string(), "Database query error");
    }

    #[test]
    fn test_error_source_is_none() {
        // DbError doesn't have a source, so it should return None
        use std::error::Error;
        let error = DbError::ConnectionError;
        println!("{}", error.source().is_none());
        assert!(error.source().is_none());
    }

    #[test]
    fn test_display_in_format_macro() {
        let error = DbError::PoolError;
        let formatted = format!("Error occurred: {}", error);
        println!("{}", formatted);
        assert_eq!(formatted, "Error occurred: Database pool error");
    }

    #[test]
    fn test_error_can_be_cloned_via_match() {
        // Since DbError doesn't derive Clone, test that we can recreate it
        let error = DbError::TransactionError;
        let error_msg = format!("{}", error);
        println!("{}", error_msg);
        assert!(error_msg.contains("transaction"));
    }

    #[test]
    fn test_all_variants_are_unique() {
        // Ensure each variant produces a unique message
        let connection = DbError::ConnectionError.to_string();
        let query = DbError::QueryError.to_string();
        let transaction = DbError::TransactionError.to_string();
        let pool = DbError::PoolError.to_string();
        println!("{}", connection);
        println!("{}", query);
        println!("{}", transaction);
        println!("{}", pool);

        assert_ne!(connection, query);
        assert_ne!(connection, transaction);
        assert_ne!(connection, pool);
        assert_ne!(query, transaction);
        assert_ne!(query, pool);
        assert_ne!(transaction, pool);
    }
}
