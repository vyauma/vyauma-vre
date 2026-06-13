//! Built-in project templates for `vre new`.
//!
//! Each template returns the set of files to write into the new project
//! directory. All file contents are inlined here (no external assets needed)
//! so the CLI binary is fully self-contained.

use super::TemplateFile;
use crate::config::vre_toml;

/// Return the list of files for the named template.
pub fn files_for(name: &str, template: &str) -> Result<Vec<TemplateFile>, String> {
    match template {
        "app"     => Ok(app_template(name)),
        "desktop" => Ok(desktop_template(name)),
        "api"     => Ok(api_template(name)),
        "library" => Ok(library_template(name)),
        "mobile"  => Ok(mobile_template(name)),
        other => Err(format!(
            "Unknown template '{other}'. Available: app, desktop, api, library, mobile\n\
             Run `vre new --help` for details."
        )),
    }
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn default_target() -> String {
    #[cfg(target_os = "windows")]
    return "windows-x64".to_string();
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x64".to_string();
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-arm64".to_string();
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "macos-x64".to_string();
    #[allow(unreachable_code)]
    "linux-x64".to_string()
}

fn gitignore() -> TemplateFile {
    TemplateFile {
        path: ".gitignore",
        content: "\
# VRE build artifacts
dist/
*.vpkg

# Dependencies
vym_modules/

# Debug
*.vbc

# Editor
.vscode/
.idea/
*.swp
".to_string(),
    }
}

fn vre_toml(name: &str, template_label: &str) -> TemplateFile {
    TemplateFile {
        path: "vre.toml",
        content: format!(
            r#"[project]
name = "{name}"
version = "0.1.0"
authors = []
description = "A Vyauma {template_label} application"

[target]
default = "{target}"

[dependencies]
# std = "1.0.0"

[capabilities]
filesystem = false
network = false
"#,
            target = default_target()
        ),
    }
}

// ── app template ─────────────────────────────────────────────────────────────

fn app_template(name: &str) -> Vec<TemplateFile> {
    vec![
        vre_toml(name, ""),
        gitignore(),
        TemplateFile {
            path: "src/main.vya",
            content: format!(
                r#"// {name} — Vyauma Application
// Getting started: https://docs.vyauma.org

fn main() {{
    print("Hello from {name}!");

    let result: Int64 = add(10, 32);
    print(result);
}}

fn add(a: Int64, b: Int64) -> Int64 {{
    return a + b;
}}
"#
            ),
        },
        TemplateFile {
            path: "README.md",
            content: format!(
                r#"# {name}

A Vyauma application.

## Getting Started

```bash
# Run the application
vre run

# Run in release mode
vre run --release

# Build for distribution
vre build

# Run tests
vre test
```

## Project Structure

```
{name}/
├── src/
│   └── main.vya      # Application entry point
├── vre.toml          # Project configuration
└── README.md
```
"#
            ),
        },
    ]
}

// ── desktop template ─────────────────────────────────────────────────────────

fn desktop_template(name: &str) -> Vec<TemplateFile> {
    vec![
        TemplateFile {
            path: "vre.toml",
            content: format!(
                r#"[project]
name = "{name}"
version = "0.1.0"
authors = []
description = "A Vyauma desktop application"

[target]
default = "{target}"

[dependencies]
# vre-ui = "1.0.0"

[capabilities]
filesystem = true
network = false
"#,
                target = default_target()
            ),
        },
        gitignore(),
        TemplateFile {
            path: "src/main.vya",
            content: format!(
                r#"// {name} — Vyauma Desktop Application
// Powered by the VRE Native UI Framework

import {{ Window, Button, Label }} from "vre-ui";

fn main() {{
    let window = Window::new("{name}");
    window.set_size(800, 600);

    let label = Label::new("Welcome to {name}!");
    label.set_position(20, 20);

    let btn = Button::new("Click Me");
    btn.set_position(20, 60);
    btn.on_click(|| {{
        label.set_text("Button clicked!");
    }});

    window.add(label);
    window.add(btn);
    window.show();
    window.run();
}}
"#
            ),
        },
        TemplateFile {
            path: "README.md",
            content: format!(
                r#"# {name}

A Vyauma native desktop application.

## Running

```bash
vre run
```

## Building

```bash
vre build --target windows-x64
vre build --target linux-x64
vre build --target macos-arm64
```
"#
            ),
        },
    ]
}

// ── api template ─────────────────────────────────────────────────────────────

fn api_template(name: &str) -> Vec<TemplateFile> {
    vec![
        TemplateFile {
            path: "vre.toml",
            content: format!(
                r#"[project]
name = "{name}"
version = "0.1.0"
authors = []
description = "A Vyauma HTTP API server"

[target]
default = "{target}"

[dependencies]
# http-router = "1.0.0"

[capabilities]
filesystem = false
network = true
"#,
                target = default_target()
            ),
        },
        gitignore(),
        TemplateFile {
            path: "src/main.vya",
            content: format!(
                r#"// {name} — Vyauma HTTP API Server

import {{ HttpServer, Request, Response }} from "std/http";

fn main() {{
    let server = HttpServer::new();

    server.get("/", |req: Request| -> Response {{
        return Response::json({{ "status": "ok", "service": "{name}" }});
    }});

    server.get("/health", |req: Request| -> Response {{
        return Response::json({{ "healthy": true }});
    }});

    print("Server listening on http://0.0.0.0:8080");
    server.listen("0.0.0.0", 8080);
}}
"#
            ),
        },
        TemplateFile {
            path: "src/routes/health.vya",
            content: r#"// Health check route

import { Response } from "std/http";

pub fn health_handler() -> Response {
    return Response::json({ "healthy": true, "uptime": 0 });
}
"#.to_string(),
        },
        TemplateFile {
            path: "README.md",
            content: format!(
                r#"# {name}

A Vyauma HTTP API server.

## Running

```bash
vre run --allow-net
```

## Building

```bash
vre build
vre deploy docker
```
"#
            ),
        },
    ]
}

// ── library template ─────────────────────────────────────────────────────────

fn library_template(name: &str) -> Vec<TemplateFile> {
    vec![
        TemplateFile {
            path: "vre.toml",
            content: format!(
                r#"[project]
name = "{name}"
version = "0.1.0"
authors = []
description = "A Vyauma library"

[target]
default = "{target}"

[dependencies]

[capabilities]
filesystem = false
network = false
"#,
                target = default_target()
            ),
        },
        gitignore(),
        TemplateFile {
            path: "src/lib.vya",
            content: format!(
                r#"// {name} — Vyauma Library
// Public API surface

/// Add two integers and return the result.
pub fn add(a: Int64, b: Int64) -> Int64 {{
    return a + b;
}}

/// Check if a value is even.
pub fn is_even(n: Int64) -> Bool {{
    return n % 2 == 0;
}}
"#
            ),
        },
        TemplateFile {
            path: "tests/test_lib.vya",
            content: format!(
                r#"// Tests for {name}

import {{ add, is_even }} from "../src/lib";

fn test_add() {{
    assert_eq(add(1, 2), 3);
    assert_eq(add(0, 0), 0);
    assert_eq(add(-1, 1), 0);
}}

fn test_is_even() {{
    assert(is_even(4));
    assert(!is_even(3));
}}
"#
            ),
        },
        TemplateFile {
            path: "README.md",
            content: format!(
                r#"# {name}

A Vyauma library.

## Usage

Add to your `vre.toml`:

```toml
[dependencies]
{name} = "0.1.0"
```

## Publishing

```bash
vre publish
```
"#
            ),
        },
    ]
}

// ── mobile template ───────────────────────────────────────────────────────────

fn mobile_template(name: &str) -> Vec<TemplateFile> {
    vec![
        TemplateFile {
            path: "vre.toml",
            content: format!(
                r#"[project]
name = "{name}"
version = "0.1.0"
authors = []
description = "A Vyauma mobile application"

[target]
default = "android-arm64"

[dependencies]
# vre-mobile-ui = "1.0.0"

[capabilities]
filesystem = true
network = true
"#
            ),
        },
        gitignore(),
        TemplateFile {
            path: "src/main.vya",
            content: format!(
                r#"// {name} — Vyauma Mobile Application

import {{ App, Screen, Text, Button }} from "vre-mobile";

fn main() {{
    let app = App::new("{name}");

    let screen = Screen::new("Home");

    let title = Text::new("Welcome to {name}!");
    title.set_style("font-size: 24; font-weight: bold;");

    let btn = Button::new("Get Started");
    btn.on_press(|| {{
        print("Button pressed!");
    }});

    screen.add(title);
    screen.add(btn);
    app.set_home(screen);
    app.run();
}}
"#
            ),
        },
        TemplateFile {
            path: "android/AndroidManifest.xml",
            content: format!(
                r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="org.vyauma.{name}">
    <application
        android:label="{name}"
        android:theme="@style/AppTheme">
        <activity android:name=".VreActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#
            ),
        },
        TemplateFile {
            path: "README.md",
            content: format!(
                r#"# {name}

A Vyauma mobile application.

## Building

```bash
# Android
vre mobile build android

# iOS
vre mobile build ios
```

## Requirements

- Android: Android SDK + NDK
- iOS: Xcode 14+ (macOS only)

Run `vre doctor` to verify your environment.
"#
            ),
        },
    ]
}
