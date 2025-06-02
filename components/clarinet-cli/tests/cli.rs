use std::{path::Path, process::Command};

use clarinet_files::{FileLocation, ProjectManifest, ProjectManifestFile};

#[track_caller]
fn parse_manifest(project_dir: &Path) -> ProjectManifest {
    let manifest_path = project_dir.join("Clarinet.toml");
    let manifest_str = std::fs::read_to_string(&manifest_path).unwrap();
    let manifest_file: ProjectManifestFile =
        toml::from_str(&manifest_str).expect("failed to parse Clarinet.toml");
    let location = FileLocation::from_path(manifest_path);
    ProjectManifest::from_project_manifest_file(manifest_file, &location, false).unwrap()
}

#[track_caller]
fn create_new_project(project_name: &str) -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let status = Command::new(env!("CARGO_BIN_EXE_clarinet"))
        .arg("new")
        .arg(project_name)
        .current_dir(&temp_dir)
        .status()
        .unwrap();
    assert!(status.success(), "clarinet new did not exit successfully");
    temp_dir
}

#[test]
fn test_new_project() {
    let project_name = "test_project";
    let temp_dir = create_new_project(project_name);
    let project_path = temp_dir.path().join(project_name);
    assert!(project_path.exists() && project_path.is_dir(),);
    let clarinet_toml = project_path.join("Clarinet.toml");
    assert!(clarinet_toml.exists() && clarinet_toml.is_file());

    let expected_start = format!("[project]\nname = \"{}\"", project_name);
    let manifest_str = std::fs::read_to_string(&clarinet_toml).unwrap();
    let first_two_lines: String = manifest_str.lines().take(2).collect::<Vec<_>>().join("\n");
    assert_eq!(first_two_lines, expected_start);
}

#[test]
fn test_contract_new() {
    let project_name = "test_contract_new";
    let temp_dir = create_new_project(project_name);
    let project_path = temp_dir.path().join(project_name);
    let contract_name = "test_contract";
    let status = Command::new(env!("CARGO_BIN_EXE_clarinet"))
        .arg("contract")
        .arg("new")
        .arg(contract_name)
        .current_dir(&project_path)
        .status()
        .unwrap();
    assert!(status.success());

    let contract_path = project_path
        .join("contracts")
        .join(format!("{}.clar", contract_name));
    assert!(contract_path.exists() && contract_path.is_file());

    let expected_start = format!(";; title: {}", contract_name);
    let contract_str = std::fs::read_to_string(&contract_path).unwrap();
    let first_line = contract_str.lines().next().unwrap();
    assert_eq!(first_line, expected_start);
}

#[test]
fn test_requirement_add() {
    let project_name = "test_requirement_add";
    let temp_dir = create_new_project(project_name);
    let project_path = temp_dir.path().join(project_name);
    let requirement_name = "SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.sip-010-trait-ft-standard";
    let status = Command::new(env!("CARGO_BIN_EXE_clarinet"))
        .arg("requirement")
        .arg("add")
        .arg(requirement_name)
        .current_dir(&project_path)
        .status()
        .unwrap();
    assert!(status.success());

    let manifest = parse_manifest(&project_path);
    let requirement = manifest
        .project
        .requirements
        .iter()
        .find(|r| r.iter().any(|c| c.contract_id == requirement_name));
    assert!(requirement.is_some());
}
