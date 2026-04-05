use std::process::Command;

// @minter:integration durable-mutations-use-emberflow-surface
#[test]
fn cargo_package_is_free_of_missing_manifest_metadata_warnings() {
    let output = Command::new("cargo")
        .args([
            "package",
            "--manifest-path",
            "Cargo.toml",
            "--allow-dirty",
            "--no-verify",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo package should execute");

    assert!(
        output.status.success(),
        "cargo package failed with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("manifest has no description"),
        "cargo package warned about missing description:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("manifest has no license"),
        "cargo package warned about missing license:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("manifest has no documentation"),
        "cargo package warned about missing documentation:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("manifest has no homepage"),
        "cargo package warned about missing homepage:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("manifest has no repository"),
        "cargo package warned about missing repository:\n{}",
        stderr
    );
}
