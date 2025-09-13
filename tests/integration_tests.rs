//! Integration tests for wol-proxy
//!
//! These tests verify that the binaries work correctly and can be executed.
//! Note: These tests do not require actual network functionality to avoid
//! flakiness in CI environments.

use std::process::Command;
use std::str;

#[test]
fn test_wol_binary_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wol", "--", "--help"])
        .output()
        .expect("Failed to execute wol binary");

    assert!(output.status.success());
    let stdout = str::from_utf8(&output.stdout).unwrap();

    // Verify key help text is present
    assert!(stdout.contains("The MAC address of the server"));
    assert!(stdout.contains("The target address (ip:port) of the server"));
    assert!(stdout.contains("The address to listen on"));
    assert!(stdout.contains("Maximum time to wait for the server to wake up"));
}

#[test]
fn test_keepawake_binary_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "keepawake", "--", "--help"])
        .output()
        .expect("Failed to execute keepawake binary");

    assert!(output.status.success());
    let stdout = str::from_utf8(&output.stdout).unwrap();

    // Verify key help text is present
    assert!(stdout.contains("TCP proxy to keep the machine awake"));
    assert!(stdout.contains("Address of the target"));
    assert!(stdout.contains("Listen address to bind to"));
    assert!(stdout.contains("Number of seconds to keep the wake lock active"));
}

#[test]
fn test_wol_binary_version() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wol", "--", "--version"])
        .output()
        .expect("Failed to execute wol binary");

    assert!(output.status.success());
    let stdout = str::from_utf8(&output.stdout).unwrap();

    // Should contain version information
    assert!(stdout.contains("wol-proxy"));
}

#[test]
fn test_keepawake_binary_version() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "keepawake", "--", "--version"])
        .output()
        .expect("Failed to execute keepawake binary");

    assert!(output.status.success());
    let stdout = str::from_utf8(&output.stdout).unwrap();

    // Should contain version information
    assert!(stdout.contains("wol-proxy"));
}

#[test]
fn test_wol_missing_arguments() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wol"])
        .output()
        .expect("Failed to execute wol binary");

    // Should fail when required arguments are missing
    assert!(!output.status.success());
    let stderr = str::from_utf8(&output.stderr).unwrap();

    // Should mention missing required arguments
    assert!(stderr.contains("required") || stderr.contains("error"));
}

#[test]
fn test_keepawake_missing_arguments() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "keepawake"])
        .output()
        .expect("Failed to execute keepawake binary");

    // Should fail when required arguments are missing
    assert!(!output.status.success());
    let stderr = str::from_utf8(&output.stderr).unwrap();

    // Should mention missing required arguments
    assert!(stderr.contains("required") || stderr.contains("error"));
}

#[test]
fn test_wol_invalid_mac_address() {
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "wol",
            "--",
            "--mac",
            "invalid",
            "--target",
            "127.0.0.1:8080",
            "--bind",
            "127.0.0.1:8081",
        ])
        .output()
        .expect("Failed to execute wol binary");

    // Should fail with invalid MAC address
    assert!(!output.status.success());
}

#[test]
fn test_wol_invalid_target_address() {
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "wol",
            "--",
            "--mac",
            "aa:bb:cc:dd:ee:ff",
            "--target",
            "invalid_address",
            "--bind",
            "127.0.0.1:8081",
        ])
        .output()
        .expect("Failed to execute wol binary");

    // Should fail with invalid target address
    assert!(!output.status.success());
}

#[test]
fn test_keepawake_invalid_target_address() {
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "keepawake",
            "--",
            "--target",
            "invalid_address",
            "--bind",
            "127.0.0.1:8081",
        ])
        .output()
        .expect("Failed to execute keepawake binary");

    // Should fail with invalid target address
    assert!(!output.status.success());
}
