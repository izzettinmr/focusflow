pub mod core;
#[cfg(feature = "desktop")]
mod desktop;
pub mod files;
pub mod sync;
#[cfg(feature = "desktop")]
pub use desktop::run;
