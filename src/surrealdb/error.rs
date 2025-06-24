use std::fmt;
use conduwuit::{Result as CoreResult, error, info, warn};

/// SurrealDB-specific error types
#[derive(Debug)]
pub enum Error {
    /// Connection-related errors
    Connection(String),
    
    /// Authentication errors
    Authentication(String),
    
    /// Query execution errors
    Query(String),
    
    /// Transaction errors
    Transaction(String),
    
    /// Schema-related errors
    Schema(String),
    
    /// Pool management errors
    Pool(String),
    
    /// Configuration errors
    Config(String),
    
    /// Timeout errors
    Timeout(String),
    
    /// Health check errors
    Health(String),
    
    /// Generic SurrealDB errors
    Surreal(surrealdb::Error),
    
    /// IO errors
    Io(std::io::Error),
    
    /// Serialization errors
    Serialization(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Connection(msg) => write!(f, "SurrealDB connection error: {msg}"),
            Error::Authentication(msg) => write!(f, "SurrealDB authentication error: {msg}"),
            Error::Query(msg) => write!(f, "SurrealDB query error: {msg}"),
            Error::Transaction(msg) => write!(f, "SurrealDB transaction error: {msg}"),
            Error::Schema(msg) => write!(f, "SurrealDB schema error: {msg}"),
            Error::Pool(msg) => write!(f, "SurrealDB pool error: {msg}"),
            Error::Config(msg) => write!(f, "SurrealDB configuration error: {msg}"),
            Error::Timeout(msg) => write!(f, "SurrealDB timeout error: {msg}"),
            Error::Health(msg) => write!(f, "SurrealDB health check error: {msg}"),
            Error::Surreal(e) => write!(f, "SurrealDB error: {e}"),
            Error::Io(e) => write!(f, "SurrealDB I/O error: {e}"),
            Error::Serialization(msg) => write!(f, "SurrealDB serialization error: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Surreal(e) => Some(e),
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<surrealdb::Error> for Error {
    fn from(e: surrealdb::Error) -> Self {
        Error::Surreal(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(e: tokio::time::error::Elapsed) -> Self {
        Error::Timeout(format!("Operation timed out: {e}"))
    }
}

// impl From<async_channel::SendError<crate::pool::PooledConnection>> for Error {
//     fn from(e: async_channel::SendError<crate::pool::PooledConnection>) -> Self {
//         Error::Pool(format!("Failed to return connection to pool: {e}"))
//     }
// }

impl From<async_channel::RecvError> for Error {
    fn from(e: async_channel::RecvError) -> Self {
        Error::Pool(format!("Failed to receive connection from pool: {e}"))
    }
}

impl From<tokio::sync::AcquireError> for Error {
    fn from(e: tokio::sync::AcquireError) -> Self {
        Error::Pool(format!("Failed to acquire connection semaphore: {e}"))
    }
}

/// Type alias for Results with SurrealDB errors
pub type Result<T> = std::result::Result<T, Error>;

/// Helper macros for creating specific error types
#[macro_export]
macro_rules! connection_error {
    ($msg:expr) => {
        $crate::error::Error::Connection($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::Error::Connection(format!($fmt, $($arg)*))
    };
}

#[macro_export]
macro_rules! auth_error {
    ($msg:expr) => {
        $crate::error::Error::Authentication($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::Error::Authentication(format!($fmt, $($arg)*))
    };
}

#[macro_export]
macro_rules! query_error {
    ($msg:expr) => {
        $crate::error::Error::Query($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::Error::Query(format!($fmt, $($arg)*))
    };
}

#[macro_export]
macro_rules! schema_error {
    ($msg:expr) => {
        $crate::error::Error::Schema($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::Error::Schema(format!($fmt, $($arg)*))
    };
}

#[macro_export]
macro_rules! pool_error {
    ($msg:expr) => {
        $crate::error::Error::Pool($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::Error::Pool(format!($fmt, $($arg)*))
    };
}

/// Helper function to convert SurrealDB errors to conduwuit errors
pub fn to_conduwuit_error(e: Error) -> conduwuit::Error {
    conduwuit::err!("{e}")
}

/// Helper function to convert conduwuit results to SurrealDB results
pub fn from_conduwuit_result<T>(result: CoreResult<T>) -> Result<T> {
    result.map_err(|e| Error::Config(e.to_string()))
}

/// Convert our Error to conduwuit::Error
impl From<Error> for conduwuit::Error {
    fn from(e: Error) -> Self {
        conduwuit::err!("{e}")
    }
}

/// Convert conduwuit::Error to our Error
impl From<conduwuit::Error> for Error {
    fn from(e: conduwuit::Error) -> Self {
        Error::Config(e.to_string())
    }
}

// Note: Cannot implement From<surrealdb::Error> for conduwuit::Error due to orphan rules
// Use explicit conversion functions instead

/// Universal error conversion helper for SurrealDB operations
pub fn convert_surreal_error<T>(result: std::result::Result<T, surrealdb::Error>) -> Result<T> {
    result.map_err(|e| Error::Surreal(e))
}

/// Helper for converting SurrealDB results to conduwuit results
pub fn to_conduwuit_result<T>(result: Result<T>) -> conduwuit::Result<T> {
    result.map_err(|e| conduwuit::err!("{e}"))
}

/// Conversion helper specifically for query operations
pub fn convert_query_result<T>(result: std::result::Result<T, surrealdb::Error>) -> Result<T> {
    result.map_err(|e| Error::Query(format!("SurrealDB query failed: {e}")))
}

/// Conversion helper specifically for connection operations  
pub fn convert_connection_result<T>(result: std::result::Result<T, surrealdb::Error>) -> Result<T> {
    result.map_err(|e| Error::Connection(format!("SurrealDB connection failed: {e}")))
}
