//! VRE CLI diagnostics system.
//!
//! Provides structured, colored error output with error codes,
//! suggestions, and fix hints — inspired by rustc and clang diagnostics.

pub mod codes;

use std::fmt;

// ── ANSI color helpers ────────────────────────────────────────────────────────

/// Returns true if ANSI color output is supported and not disabled.
fn color_enabled() -> bool {
    // Respect NO_COLOR convention (https://no-color.org)
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    // Respect TERM=dumb
    if std::env::var("TERM").unwrap_or_default() == "dumb" {
        return false;
    }
    true
}

macro_rules! ansi {
    ($code:expr, $text:expr) => {
        if color_enabled() {
            format!("\x1b[{}m{}\x1b[0m", $code, $text)
        } else {
            $text.to_string()
        }
    };
}

fn red(s: &str)     -> String { ansi!("1;31", s) }
fn yellow(s: &str)  -> String { ansi!("1;33", s) }
fn cyan(s: &str)    -> String { ansi!("36", s) }
fn bold(s: &str)    -> String { ansi!("1", s) }
fn dim(s: &str)     -> String { ansi!("2", s) }
fn green(s: &str)   -> String { ansi!("1;32", s) }

// ── Diagnostic severity ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error   => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Note    => write!(f, "note"),
        }
    }
}

// ── Diagnostic struct ─────────────────────────────────────────────────────────

/// A structured diagnostic message.
///
/// # Example
/// ```
/// use crate::diagnostics::{Diagnostic, Severity};
/// use crate::diagnostics::codes::E001;
///
/// Diagnostic::new(Severity::Error, E001, "Package 'foo' not found")
///     .with_suggestion("Run: vre search foo")
///     .emit();
/// ```
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub suggestion: Option<String>,
    pub hint: Option<String>,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub col: Option<usize>,
    pub source_line: Option<String>,
}

impl Diagnostic {
    /// Create a new diagnostic.
    pub fn new(severity: Severity, code: &'static str, message: impl Into<String>) -> Self {
        Diagnostic {
            severity,
            code,
            message: message.into(),
            suggestion: None,
            hint: None,
            file: None,
            line: None,
            col: None,
            source_line: None,
        }
    }

    /// Create an error-level diagnostic.
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, message)
    }

    /// Create a warning-level diagnostic.
    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, message)
    }

    /// Attach a suggestion (shown under "Suggestion:").
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Attach a fix hint (shown under "Hint:").
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Attach source location.
    pub fn with_location(mut self, file: impl Into<String>, line: usize, col: usize) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self.col = Some(col);
        self
    }

    /// Attach the source line text for inline display.
    pub fn with_source_line(mut self, line: impl Into<String>) -> Self {
        self.source_line = Some(line.into());
        self
    }

    /// Print this diagnostic to stderr.
    pub fn emit(&self) {
        let label = match self.severity {
            Severity::Error   => red(&format!("{}[{}]", self.severity, self.code)),
            Severity::Warning => yellow(&format!("{}[{}]", self.severity, self.code)),
            Severity::Note    => cyan(&format!("{}[{}]", self.severity, self.code)),
        };

        eprintln!();
        eprintln!("{}: {}", label, bold(&self.message));

        if let (Some(file), Some(line), Some(col)) = (&self.file, self.line, self.col) {
            eprintln!("  {} {}:{}:{}", dim("-->"), file, line, col);
        } else if let Some(file) = &self.file {
            eprintln!("  {} {}", dim("-->"), file);
        }

        if let (Some(src), Some(line)) = (&self.source_line, self.line) {
            let col = self.col.unwrap_or(1);
            let padding = " ".repeat(col.saturating_sub(1));
            eprintln!("   {}", dim("|"));
            eprintln!("{:>3} {} {}", line, dim("|"), src);
            eprintln!("   {} {}{}",  dim("|"), padding, red("^"));
            eprintln!("   {}", dim("|"));
        }

        if let Some(suggestion) = &self.suggestion {
            eprintln!();
            eprintln!("  {}", bold("Suggestion:"));
            for line in suggestion.lines() {
                eprintln!("    {}", cyan(line));
            }
        }

        if let Some(hint) = &self.hint {
            eprintln!();
            eprintln!("  {}: {}", bold("Hint"), hint);
        }

        eprintln!();
    }
}

// ── Compiler error renderer ───────────────────────────────────────────────────

/// Render a compiler error string (format: `[line:col] message`) as a
/// properly formatted diagnostic. Preserves all existing diagnostic output
/// behavior from the original `main.rs::render_diagnostic()`.
pub fn emit_compiler_error(source: &str, filename: &str, error: &str) {
    if error.starts_with('[') {
        if let Some(close) = error.find(']') {
            let span_part = &error[1..close];
            let rest = error[close + 1..].trim();
            let parts: Vec<&str> = span_part.splitn(2, ':').collect();
            if parts.len() == 2 {
                if let (Ok(line_num), Ok(col_num)) =
                    (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                {
                    let lines: Vec<&str> = source.lines().collect();
                    let source_line = if line_num > 0 && line_num <= lines.len() {
                        lines[line_num - 1].to_string()
                    } else {
                        String::new()
                    };

                    Diagnostic::error(codes::E005, rest)
                        .with_location(filename, line_num, col_num)
                        .with_source_line(source_line)
                        .emit();
                    return;
                }
            }
        }
    }

    // Fallback: plain error (type errors, etc.)
    Diagnostic::error(codes::E005, error)
        .with_location(filename, 0, 0)
        .emit();
}

// ── Doctor check helpers ──────────────────────────────────────────────────────

/// Print a passing check line.
pub fn check_pass(label: &str) {
    eprintln!("  {} {}", green("✓"), bold(label));
}

/// Print a failing check line with an optional suggestion.
pub fn check_fail(label: &str, suggestion: Option<&str>) {
    eprintln!("  {} {}", red("✗"), bold(label));
    if let Some(s) = suggestion {
        eprintln!("      {}", dim(s));
    }
}

/// Print a warning check line.
pub fn check_warn(label: &str, detail: &str) {
    eprintln!("  {} {} — {}", yellow("⚠"), bold(label), dim(detail));
}

/// Print a section header for the doctor output.
pub fn section(title: &str) {
    eprintln!();
    eprintln!("  {}", bold(&format!("[ {} ]", title)));
    eprintln!();
}
