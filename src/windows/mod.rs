//! These modules are all glue to support reading the MSVC version from
//! the registry and from COM interfaces.

// This is used in the crate's public API, so don't use #[cfg(windows)]
pub mod find_tools;

#[cfg(windows)]
pub(crate) mod windows_sys;
#[cfg(windows)]
mod windows_targets;
