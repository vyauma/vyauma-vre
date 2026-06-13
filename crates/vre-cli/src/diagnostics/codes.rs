//! VRE CLI error codes.
//!
//! All diagnostic error codes are defined here as `&'static str` constants
//! so they can be embedded in [`super::Diagnostic`] without allocation.

/// Package not found in the registry.
///
/// Suggestion: Run `vre search <package-name>` to find available packages.
pub const E001: &str = "E001";

/// Invalid or unrecognised package version specifier.
///
/// Suggestion: Use `name@x.y.z` or `name@latest`.
pub const E002: &str = "E002";

/// `vre.toml` not found in the current or any ancestor directory.
///
/// Suggestion: Run `vre init` to create a project configuration file.
pub const E003: &str = "E003";

/// VRE Registry is unreachable.
///
/// Suggestion: Check your network connection or try again later.
pub const E004: &str = "E004";

/// Source compilation failed.
///
/// The compiler returned an error for the provided source file.
pub const E005: &str = "E005";

/// Runtime execution error.
///
/// The VRE virtual machine encountered an unrecoverable error.
pub const E006: &str = "E006";

/// Capability denied.
///
/// The program attempted to use a capability not granted at startup.
/// Suggestion: Run with the appropriate `--allow-*` flag.
pub const E007: &str = "E007";

/// Invalid or unsupported build target platform.
///
/// Suggestion: Run `vre build --help` for a list of supported targets.
pub const E008: &str = "E008";

/// A required dependency is missing.
///
/// Suggestion: Run `vre install` to install all project dependencies.
pub const E009: &str = "E009";

/// Build output could not be written to the output directory.
///
/// Suggestion: Check directory permissions and available disk space.
pub const E010: &str = "E010";

/// Project template not found or invalid.
///
/// Suggestion: Run `vre new --help` for a list of built-in templates.
pub const E011: &str = "E011";

/// Package signature verification failed.
///
/// The downloaded package signature does not match the registry record.
pub const E012: &str = "E012";

/// Mobile SDK not found.
///
/// Android SDK or Xcode is required for mobile builds.
/// Suggestion: Run `vre doctor` to diagnose your environment.
pub const E013: &str = "E013";

/// Source file not found.
///
/// The specified source file does not exist or is not readable.
pub const E014: &str = "E014";

/// Project already initialised.
///
/// A `vre.toml` already exists in this directory.
pub const E015: &str = "E015";
