use std::fs;
use std::path::Path;

pub fn pack_ios(input_path: &str) {
    let build_dir = Path::new("build/ios");
    if let Err(e) = fs::create_dir_all(build_dir) {
        println!("Failed to create ios build directory: {}", e);
        return;
    }
    
    // Copy the VYM file to resources
    let res_dir = build_dir.join("Resources");
    fs::create_dir_all(&res_dir).unwrap();
    let dest_file = res_dir.join("app.vym");
    if let Err(e) = fs::copy(input_path, &dest_file) {
        println!("Could not copy input file to resources (might need compilation first): {}", e);
    }
    
    // Info.plist
    let plist_path = build_dir.join("Info.plist");
    let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>org.vyauma.vre.app</string>
    <key>CFBundleName</key>
    <string>VRE App</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>UILaunchStoryboardName</key>
    <string>LaunchScreen</string>
</dict>
</plist>
"#;
    fs::write(plist_path, plist_content).unwrap();

    // AppDelegate.swift
    let source_dir = build_dir.join("Sources");
    fs::create_dir_all(&source_dir).unwrap();
    let app_delegate_path = source_dir.join("AppDelegate.swift");
    let app_delegate_content = r#"import UIKit

@main
class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?

    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        // Init VRE VM bridging here
        runVreApp("app.vym")
        
        window = UIWindow(frame: UIScreen.main.bounds)
        window?.rootViewController = UIViewController()
        window?.rootViewController?.view.backgroundColor = .white
        window?.makeKeyAndVisible()
        return true
    }
    
    // C bridge to Rust VRE Core
    func runVreApp(_ file: String) {
        // vre_ios_run(file)
    }
}
"#;
    fs::write(app_delegate_path, app_delegate_content).unwrap();

    println!("iOS project scaffold generated at build/ios/");
}
