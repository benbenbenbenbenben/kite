use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn kite_bin() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/debug/kite");
    if !path.exists() {
        path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../../target/release/kite");
    }
    path
}

#[test]
fn test_cli_human_format() {
    let temp_dir = TempDir::new().unwrap();
    let kite_path = temp_dir.path().join("main.kite");
    std::fs::write(&kite_path, "context Test {}").unwrap();

    let output = Command::new(kite_bin())
        .arg("check")
        .arg(&kite_path)
        .arg("--format")
        .arg("human")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All contexts verified"));
    assert!(stdout.contains("1 context(s) parsed"));
}

#[test]
fn test_cli_tap_format() {
    let temp_dir = TempDir::new().unwrap();
    let kite_path = temp_dir.path().join("main.kite");
    std::fs::write(&kite_path, "context Test {}").unwrap();

    let output = Command::new(kite_bin())
        .arg("check")
        .arg(&kite_path)
        .arg("--format")
        .arg("tap")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("TAP version 13"));
    assert!(stdout.contains("ok 1 - all contexts verified"));
}

#[test]
fn test_cli_ctrf_format() {
    let temp_dir = TempDir::new().unwrap();
    let kite_path = temp_dir.path().join("main.kite");
    std::fs::write(&kite_path, "context Test {}").unwrap();

    let output = Command::new(kite_bin())
        .arg("check")
        .arg(&kite_path)
        .arg("--format")
        .arg("ctrf")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["results"]["tool"]["name"], "kite");
    assert_eq!(json["results"]["summary"]["passed"], 1);
}

#[test]
fn test_cli_junit_format() {
    let temp_dir = TempDir::new().unwrap();
    let kite_path = temp_dir.path().join("main.kite");
    std::fs::write(&kite_path, "context Test {}").unwrap();

    let output = Command::new(kite_bin())
        .arg("check")
        .arg(&kite_path)
        .arg("--format")
        .arg("junit")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<?xml"));
    assert!(stdout.contains("<testsuite name=\"kite\""));
    assert!(stdout.contains("<testcase name=\"verification\""));
}
