//! How Core reaches what is outside it: the way it writes its files, and the way it asks the
//! network.
pub mod atomic_file;
pub mod bounded_read;
pub mod http;
pub mod retry_after;
pub(crate) mod wipe;
