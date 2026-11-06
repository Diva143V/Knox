//! Concrete model provider adapters.

pub mod cloud;
pub mod local;

pub use cloud::CloudAdapter;
pub use local::LocalAdapter;

