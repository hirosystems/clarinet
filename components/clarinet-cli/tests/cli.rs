use std::{fs, path::Path, process::Command};

use clarinet_files::{FileLocation, ProjectManifest, ProjectManifestFile};

#[track_caller]
fn parse_manifest(project_dir: &Path) -> ProjectManifest {
    let manifest_path = project_dir.join("Clarinet.toml");
    let manifest_str = fs::read_to_string(&manifest_path).expect("Failed to read Clarinet.toml");
    let manifest_file: ProjectManifestFile = toml::from_str(&manifest_str).unwrap();
    let location = FileLocation::from_path(manifest_path);
    ProjectManifest::from_project_manifest_file(manifest_file, &location, false).unwrap()
}

#[track_caller]
fn create_new_project(project_name: &str) -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let status = Command::new(env!("CARGO_BIN_EXE_clarinet"))
        .args(["new", project_name])
        .current_dir(&temp_dir)
        .status();
    assert!(status.unwrap().success());
    temp_dir
}

#[test]
fn test_new_project() {
    let project_name = "test_project";
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let cmd = Command::new(env!("CARGO_BIN_EXE_clarinet"))
        .args(["new", project_name])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute clarinet new");
    assert!(cmd.status.success());

    let stdout = String::from_utf8_lossy(&cmd.stdout);
    let expected_lines = [
        "Created directory test_project",
        "Created directory contracts",
        "Created directory settings",
        "Created directory tests",
        "Created file Clarinet.toml",
        "Created file settings/Mainnet.toml",
        "Created file settings/Testnet.toml",
        "Created file settings/Devnet.toml",
        "Created directory .vscode",
        "Created file .vscode/settings.json",
        "Created file .vscode/tasks.json",
        "Created file .gitignore",
        "Created file .gitattributes",
        "Created file package.json",
        "Created file tsconfig.json",
        "Created file vitest.config.js",
    ];
    let stdout_lines: Vec<_> = stdout.lines().map(str::trim).collect();
    let expected_len = expected_lines.len();
    let actual_tail = &stdout_lines[stdout_lines.len() - expected_len..];
    assert_eq!(actual_tail, expected_lines);

    let project_path = temp_dir.path().join(project_name);
    assert!(project_path.is_dir(), "Project directory missing");
    let clarinet_toml = project_path.join("Clarinet.toml");
    assert!(clarinet_toml.is_file(), "Clarinet.toml missing");

    let manifest_str = fs::read_to_string(&clarinet_toml).expect("Failed to read Clarinet.toml");
    let expected = format!("[project]\nname = \"{}\"", project_name);
    let actual = manifest_str.lines().take(2).collect::<Vec<_>>().join("\n");
    assert_eq!(actual, expected, "Clarinet.toml header mismatch");
}

#[test]
fn test_contract_new() {
    let project_name = "test_contract_new";
    let temp_dir = create_new_project(project_name);
    let project_path = temp_dir.path().join(project_name);
    let contract_name = "test_contract";
    let output = Command::new(env!("CARGO_BIN_EXE_clarinet"))
        .args(["contract", "new", contract_name])
        .current_dir(&project_path)
        .output()
        .unwrap();
    assert!(output.status.success(), "clarinet contract new failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected_lines = [
        format!("Created file contracts/{}.clar", contract_name),
        format!("Created file tests/{}.test.ts", contract_name),
        format!("Updated Clarinet.toml with contract {}", contract_name),
    ];
    let stdout_lines: Vec<_> = stdout.lines().map(str::trim).collect();
    let expected_len = expected_lines.len();
    let actual_tail = &stdout_lines[stdout_lines.len() - expected_len..];
    assert_eq!(actual_tail, expected_lines);

    let contract_path = project_path
        .join("contracts")
        .join(format!("{contract_name}.clar"));
    assert!(contract_path.is_file(), "Contract file missing");

    let contract_str = fs::read_to_string(&contract_path).expect("Failed to read contract file");
    let expected = format!(";; title: {}", contract_name);
    assert_eq!(contract_str.lines().next().unwrap_or(""), expected);
}

#[test]
fn test_requirement_add() {
    let project_name = "test_requirement_add";
    let temp_dir = create_new_project(project_name);
    let project_path = temp_dir.path().join(project_name);
    let requirement_name = "SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.sip-010-trait-ft-standard";
    let status = Command::new(env!("CARGO_BIN_EXE_clarinet"))
        .args(["requirement", "add", requirement_name])
        .current_dir(&project_path)
        .status();
    assert!(status.unwrap().success());

    let manifest = parse_manifest(&project_path);
    let found = manifest
        .project
        .requirements
        .iter()
        .flatten()
        .any(|c| c.contract_id == requirement_name);
    assert!(found, "Requirement not found in manifest");
}
