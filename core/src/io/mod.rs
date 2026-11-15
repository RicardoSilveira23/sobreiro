//! How Core reaches what is outside it: the way it writes its files, and the way it asks the
//! network.
pub mod atomic_file;
pub mod http;
pub mod retry_after;
