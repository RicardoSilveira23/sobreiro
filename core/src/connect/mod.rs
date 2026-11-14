//! Connect and Disconnect. Only the states a connection can be in are here so far, because the
//! list of limits asks for one; the rest arrives with its own area.
pub mod connection;

pub use connection::ConnectionState;
