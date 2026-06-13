//! Template system for `vre new`.

pub mod builtin;

use std::path::Path;

/// A generated file within a project template.
pub struct TemplateFile {
    /// Relative path within the new project directory.
    pub path: &'static str,
    /// File contents.
    pub content: String,
}

/// Generate a new project in `dest` from the named template.
///
/// `name` is the project name (used in file contents).
/// `template` must match one of the built-in template names.
pub fn generate(name: &str, template: &str, dest: &Path) -> Result<(), String> {
    let files = builtin::files_for(name, template)?;

    // Create directory structure
    std::fs::create_dir_all(dest)
        .map_err(|e| format!("Failed to create directory {}: {}", dest.display(), e))?;

    for file in files {
        let target = dest.join(file.path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
        std::fs::write(&target, &file.content)
            .map_err(|e| format!("Failed to write {}: {}", target.display(), e))?;
    }

    Ok(())
}
