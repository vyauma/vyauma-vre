//! `vre new` — Create a new project from a template.

use std::path::Path;
use crate::cli::{NewArgs, Template};
use crate::diagnostics::{self, codes, Diagnostic, Severity};
use crate::templates;

pub fn run(args: NewArgs) {
    let name = &args.name;
    let template = args.template.to_string();
    let dest = Path::new(name);

    println!();
    println!("  Creating {} project '{}'...", template, name);
    println!();

    if dest.exists() {
        Diagnostic::error(codes::E015, format!("Directory '{}' already exists.", name))
            .with_hint("Choose a different name or remove the existing directory.")
            .emit();
        std::process::exit(1);
    }

    match templates::generate(name, &template, dest) {
        Ok(()) => {
            println!("  ✓ Generated project scaffold");
            println!("  ✓ Created vre.toml");
            println!("  ✓ Created src/");
            println!("  ✓ Created README.md");
            println!("  ✓ Created .gitignore");
            println!();
            println!("  Project '{}' created successfully!", name);
            println!();
            println!("  Next steps:");
            println!();
            println!("    cd {}", name);
            println!("    vre run");
            println!();
        }
        Err(e) => {
            Diagnostic::error(codes::E011, e).emit();
            std::process::exit(1);
        }
    }
}
