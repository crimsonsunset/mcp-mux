use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let config_path = Path::new("tauri.conf.json");
    if let Ok(contents) = fs::read_to_string(config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
            if let Some(identifier) = json.get("identifier").and_then(|v| v.as_str()) {
                println!("cargo:rustc-env=TAURI_APP_IDENTIFIER={}", identifier);
            }
        }
    }
    println!("cargo:rerun-if-changed=tauri.conf.json");

    // Stamp the current git HEAD into the binary so the admin UI can detect a
    // stale SPA build (web-admin serves a pre-built bundle from `apps/desktop/dist`,
    // which drifts if you change UI code without re-running `pnpm build:web:admin`).
    let git_sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    println!("cargo:rustc-env=MCPMUX_BUILD_GIT_SHA={}", git_sha);
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../../.git/logs/HEAD");

    tauri_build::build()
}
