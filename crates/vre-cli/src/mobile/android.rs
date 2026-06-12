use std::fs;
use std::path::Path;

pub fn pack_android(input_path: &str) {
    let build_dir = Path::new("build/android");
    if let Err(e) = fs::create_dir_all(build_dir) {
        println!("Failed to create android build directory: {}", e);
        return;
    }
    
    // Copy the VYM file (simulate compilation / copying)
    let assets_dir = build_dir.join("src/main/assets");
    fs::create_dir_all(&assets_dir).unwrap();
    // For now we just write a dummy byte string or copy the file
    let dest_file = assets_dir.join("app.vym");
    if let Err(e) = fs::copy(input_path, &dest_file) {
        println!("Could not copy input file to assets (might need compilation first): {}", e);
    }
    
    // AndroidManifest.xml
    let manifest_path = build_dir.join("src/main/AndroidManifest.xml");
    let manifest_content = r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="org.vyauma.vre.app">
    <uses-permission android:name="android.permission.INTERNET" />
    <application
        android:allowBackup="true"
        android:label="VRE App"
        android:theme="@style/Theme.AppCompat.Light.NoActionBar">
        <activity android:name=".MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#;
    fs::write(manifest_path, manifest_content).unwrap();

    // MainActivity.java
    let java_dir = build_dir.join("src/main/java/org/vyauma/vre/app");
    fs::create_dir_all(&java_dir).unwrap();
    let main_activity_path = java_dir.join("MainActivity.java");
    let main_activity_content = r#"package org.vyauma.vre.app;

import android.app.Activity;
import android.os.Bundle;

public class MainActivity extends Activity {
    static {
        System.loadLibrary("vre_core");
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        // Start VRE VM
        runVreApp("app.vym");
    }

    private native void runVreApp(String assetPath);
}
"#;
    fs::write(main_activity_path, main_activity_content).unwrap();

    // build.gradle
    let gradle_path = build_dir.join("build.gradle");
    let gradle_content = r#"plugins {
    id 'com.android.application'
}
android {
    compileSdkVersion 33
    defaultConfig {
        applicationId "org.vyauma.vre.app"
        minSdkVersion 21
        targetSdkVersion 33
        versionCode 1
        versionName "1.0"
    }
}
dependencies {
}
"#;
    fs::write(gradle_path, gradle_content).unwrap();

    println!("Android project scaffold generated at build/android/");
}
