//! VRE CLI — Clap command tree.
//!
//! All commands, subcommands, and flags are defined here using clap's
//! derive API. The `run()` function is the sole public entry point
//! called from `main.rs`.

use clap::{Parser, Subcommand, Args, ValueEnum};

use crate::commands;

/// Vyauma Runtime Engine — Developer Toolchain
///
/// The unified CLI for creating, building, testing, running, packaging,
/// publishing, and deploying Vyauma applications across all platforms.
#[derive(Parser, Debug)]
#[command(
    name = "vre",
    version,
    author,
    about = "Vyauma Runtime Engine — Developer CLI",
    long_about = concat!(
        "Vyauma Runtime Engine (VRE) CLI\n\n",
        "The primary developer entry point into the Vyauma ecosystem.\n",
        "Manages projects, compilation, bytecode, runtime execution,\n",
        "testing, packaging, publishing, registry interaction,\n",
        "debugging, mobile builds, cloud deployment, and more.\n\n",
        "VRE version: ", env!("CARGO_PKG_VERSION"),
    ),
    propagate_version = true,
    subcommand_required = true,
    arg_required_else_help = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    // ── Project Lifecycle ──────────────────────────────────────────────────────

    /// Create a new Vyauma project from a template
    New(NewArgs),

    /// Initialize a Vyauma project in the current directory
    Init(InitArgs),

    // ── Execution ─────────────────────────────────────────────────────────────

    /// Compile and run a Vyauma source file or project
    Run(RunArgs),

    /// Type-check a source file without executing it
    Check(CheckArgs),

    // ── Build ─────────────────────────────────────────────────────────────────

    /// Build the project for a target platform
    Build(BuildArgs),

    /// Build the project as a WebAssembly module
    BuildWeb(BuildWebArgs),

    // ── Testing ───────────────────────────────────────────────────────────────

    /// Run the project's test suite
    Test(TestArgs),

    // ── Packaging & Distribution ──────────────────────────────────────────────

    /// Package the project into a distributable `.vpkg` archive
    Package(PackageArgs),

    /// Publish the package to the VRE Registry
    Publish(PublishArgs),

    // ── Dependency Management ─────────────────────────────────────────────────

    /// Install a package from the VRE Registry
    Install(InstallArgs),

    /// Uninstall a locally installed package
    Uninstall(UninstallArgs),

    /// Upgrade the VRE toolchain and runtime
    Upgrade,

    // ── Registry ──────────────────────────────────────────────────────────────

    /// Search the VRE Registry for packages
    Search(SearchArgs),

    /// Show detailed information about a registry package
    Info(InfoArgs),

    // ── Platform Targets ──────────────────────────────────────────────────────

    /// Mobile platform build commands
    Mobile(MobileArgs),

    /// Deploy the project to a cloud or container target
    Deploy(DeployArgs),

    /// Flash firmware to an embedded device
    Flash(FlashArgs),

    // ── Developer Tooling ─────────────────────────────────────────────────────

    /// Launch the interactive debugger (DAP-compatible)
    Debug(DebugArgs),

    /// Start the Language Server Protocol (LSP) server
    Lsp,

    /// Start the Debug Adapter Protocol (DAP) server
    Dap,

    /// Generate documentation for the current project
    Doc(DocArgs),

    /// Profile a Vyauma program's execution
    Profile(ProfileArgs),

    // ── Diagnostics ───────────────────────────────────────────────────────────

    /// Diagnose the VRE installation and environment
    Doctor,

    /// Print version information for all VRE components
    Version,
}

// ── Argument Structs ────────────────────────────────────────────────────────

/// Arguments for `vre new`
#[derive(Args, Debug)]
pub struct NewArgs {
    /// Name of the new project (creates a directory with this name)
    pub name: String,

    /// Project template to use
    #[arg(long, short = 't', value_enum, default_value_t = Template::App)]
    pub template: Template,
}

/// Arguments for `vre init`
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Project name (defaults to current directory name)
    #[arg(long)]
    pub name: Option<String>,
}

/// Arguments for `vre run`
#[derive(Args, Debug)]
pub struct RunArgs {
    /// Source file to compile and run (uses project entry point if omitted)
    pub file: Option<String>,

    /// Build and run in release mode with optimizations
    #[arg(long)]
    pub release: bool,

    /// Allow filesystem read access
    #[arg(long)]
    pub allow_read: bool,

    /// Allow filesystem write access
    #[arg(long)]
    pub allow_write: bool,

    /// Allow network access
    #[arg(long)]
    pub allow_net: bool,

    /// Allow environment variable access
    #[arg(long)]
    pub allow_env: bool,

    /// Allow spawning subprocesses
    #[arg(long)]
    pub allow_run: bool,

    /// Allow database access
    #[arg(long)]
    pub allow_db: bool,

    /// Grant all capabilities (equivalent to all --allow-* flags)
    #[arg(long)]
    pub allow_all: bool,

    /// Check for heap memory leaks after execution
    #[arg(long)]
    pub check_leaks: bool,

    /// Start a distributed cluster node bound to this address
    #[arg(long, value_name = "ADDR")]
    pub cluster: Option<String>,
}

/// Arguments for `vre check`
#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Source file to type-check
    pub file: String,
}

/// Arguments for `vre build`
#[derive(Args, Debug)]
pub struct BuildArgs {
    /// Target platform
    #[arg(long, short = 't', value_enum, default_value_t = BuildTarget::WindowsX64)]
    pub target: BuildTarget,

    /// Build in release mode with optimizations
    #[arg(long)]
    pub release: bool,

    /// Output directory (defaults to `dist/`)
    #[arg(long, default_value = "dist")]
    pub out_dir: String,
}

/// Arguments for `vre build-web`
#[derive(Args, Debug)]
pub struct BuildWebArgs {
    /// Source file to compile to WebAssembly
    pub file: String,
}

/// Arguments for `vre test`
#[derive(Args, Debug)]
pub struct TestArgs {
    /// Path to a test file or directory (defaults to current project)
    pub path: Option<String>,

    /// Watch for file changes and re-run tests
    #[arg(long, short = 'w')]
    pub watch: bool,

    /// Collect and report code coverage
    #[arg(long)]
    pub coverage: bool,

    /// Filter tests by name substring
    #[arg(long, value_name = "PATTERN")]
    pub filter: Option<String>,
}

/// Arguments for `vre package`
#[derive(Args, Debug)]
pub struct PackageArgs {
    /// Output path for the .vpkg file (defaults to `<name>-<version>.vpkg`)
    #[arg(long, short = 'o')]
    pub output: Option<String>,
}

/// Arguments for `vre publish`
#[derive(Args, Debug)]
pub struct PublishArgs {
    /// Registry URL override
    #[arg(long)]
    pub registry: Option<String>,

    /// Publish without confirmation prompt
    #[arg(long)]
    pub yes: bool,
}

/// Arguments for `vre install`
#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Package to install. Use `name@version` to pin a version.
    /// Omit to install all dependencies from vre.toml.
    pub package: Option<String>,

    /// Registry URL override
    #[arg(long)]
    pub registry: Option<String>,
}

/// Arguments for `vre uninstall`
#[derive(Args, Debug)]
pub struct UninstallArgs {
    /// Package name to remove
    pub package: String,
}

/// Arguments for `vre search`
#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Registry URL override
    #[arg(long)]
    pub registry: Option<String>,

    /// Maximum number of results to display
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}

/// Arguments for `vre info`
#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Package name to look up
    pub package: String,

    /// Registry URL override
    #[arg(long)]
    pub registry: Option<String>,
}

/// Arguments for `vre mobile`
#[derive(Args, Debug)]
pub struct MobileArgs {
    #[command(subcommand)]
    pub subcommand: MobileCommand,
}

#[derive(Subcommand, Debug)]
pub enum MobileCommand {
    /// Build a mobile application package
    Build(MobileBuildArgs),
    /// Sign a mobile application package
    Sign(MobileSignArgs),
}

#[derive(Args, Debug)]
pub struct MobileBuildArgs {
    /// Mobile platform target
    #[arg(value_enum)]
    pub platform: MobilePlatform,

    /// Source file or project directory
    pub file: Option<String>,
}

#[derive(Args, Debug)]
pub struct MobileSignArgs {
    /// Mobile platform target
    #[arg(value_enum)]
    pub platform: MobilePlatform,

    /// Path to the package to sign
    pub package: String,
}

/// Arguments for `vre deploy`
#[derive(Args, Debug)]
pub struct DeployArgs {
    /// Deployment target environment
    #[arg(value_enum, default_value_t = DeployTarget::Docker)]
    pub target: DeployTarget,

    /// Source file or project directory
    pub file: Option<String>,
}

/// Arguments for `vre flash`
#[derive(Args, Debug)]
pub struct FlashArgs {
    /// Source file to compile and flash
    pub file: String,

    /// Embedded target device
    #[arg(long, short = 't', value_enum, default_value_t = EmbeddedTarget::Esp32)]
    pub target: EmbeddedTarget,
}

/// Arguments for `vre debug`
#[derive(Args, Debug)]
pub struct DebugArgs {
    /// Source file to debug (uses project entry point if omitted)
    pub file: Option<String>,

    /// DAP server port to listen on
    #[arg(long, default_value_t = 5678)]
    pub port: u16,
}

/// Arguments for `vre doc`
#[derive(Args, Debug)]
pub struct DocArgs {
    /// Project directory to generate docs for (defaults to current directory)
    pub dir: Option<String>,

    /// Output directory for generated documentation
    #[arg(long, default_value = "docs-out")]
    pub out: String,
}

/// Arguments for `vre profile`
#[derive(Args, Debug)]
pub struct ProfileArgs {
    /// Source file to profile
    pub file: String,
}

// ── Value Enums (clap-displayable) ──────────────────────────────────────────

#[derive(ValueEnum, Debug, Clone)]
pub enum Template {
    /// Standard application (default)
    App,
    /// Native desktop GUI application
    Desktop,
    /// HTTP API server
    Api,
    /// Reusable library / package
    Library,
    /// Android/iOS mobile application
    Mobile,
}

impl std::fmt::Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Template::App => write!(f, "app"),
            Template::Desktop => write!(f, "desktop"),
            Template::Api => write!(f, "api"),
            Template::Library => write!(f, "library"),
            Template::Mobile => write!(f, "mobile"),
        }
    }
}

#[derive(ValueEnum, Debug, Clone)]
pub enum BuildTarget {
    #[value(name = "windows-x64")]
    WindowsX64,
    #[value(name = "linux-x64")]
    LinuxX64,
    #[value(name = "macos-arm64")]
    MacosArm64,
    #[value(name = "macos-x64")]
    MacosX64,
    #[value(name = "android-arm64")]
    AndroidArm64,
    #[value(name = "wasm32")]
    Wasm32,
}

impl std::fmt::Display for BuildTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildTarget::WindowsX64  => write!(f, "windows-x64"),
            BuildTarget::LinuxX64    => write!(f, "linux-x64"),
            BuildTarget::MacosArm64  => write!(f, "macos-arm64"),
            BuildTarget::MacosX64    => write!(f, "macos-x64"),
            BuildTarget::AndroidArm64 => write!(f, "android-arm64"),
            BuildTarget::Wasm32      => write!(f, "wasm32"),
        }
    }
}

#[derive(ValueEnum, Debug, Clone)]
pub enum MobilePlatform {
    Android,
    Ios,
}

impl std::fmt::Display for MobilePlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MobilePlatform::Android => write!(f, "android"),
            MobilePlatform::Ios     => write!(f, "ios"),
        }
    }
}

#[derive(ValueEnum, Debug, Clone)]
pub enum DeployTarget {
    Cloud,
    Docker,
    Kubernetes,
}

impl std::fmt::Display for DeployTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeployTarget::Cloud      => write!(f, "cloud"),
            DeployTarget::Docker     => write!(f, "docker"),
            DeployTarget::Kubernetes => write!(f, "kubernetes"),
        }
    }
}

#[derive(ValueEnum, Debug, Clone)]
pub enum EmbeddedTarget {
    Esp32,
    Rpi,
    Arm,
}

impl std::fmt::Display for EmbeddedTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddedTarget::Esp32 => write!(f, "esp32"),
            EmbeddedTarget::Rpi   => write!(f, "rpi"),
            EmbeddedTarget::Arm   => write!(f, "arm"),
        }
    }
}

// ── Entry Point ──────────────────────────────────────────────────────────────

/// Parse CLI arguments and dispatch to the appropriate command handler.
///
/// This is the sole entry point called from `main()`.
pub fn run() {
    let cli = Cli::parse();
    commands::dispatch(cli.command);
}
