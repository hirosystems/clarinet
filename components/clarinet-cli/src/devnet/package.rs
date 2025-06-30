use serde::{Deserialize, Serialize};
use std::fs::File;
use std::process;

use clarinet_deployments::get_default_deployment_path;
use clarinet_deployments::types::DeploymentSpecification;
use clarinet_files::StacksNetwork;
use clarinet_files::{NetworkManifest, ProjectManifest};

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigurationPackage {
    pub deployment_plan: DeploymentSpecification,
    network_manifest: NetworkManifest,
    project_manifest: ProjectManifest,
}

fn pack_to_file(file_name: &str, package: ConfigurationPackage) -> Result<(), String> {
    let file = match File::create(file_name) {
        Ok(file) => file,
        Err(e) => {
            println!(
                "{} Unable to create file {}: {}",
                red!("error:"),
                file_name,
                e
            );
            process::exit(1);
        }
    };

    serde_json::to_writer(file, &package)
        .map_err(|e| format!("Unable to generate the json file: {e}"))?;
    println!("{file_name} file generated with success");
    Ok(())
}

fn pack_to_stdout(package: ConfigurationPackage) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&package)
        .map_err(|e| format!("failed to serialize package: {e}"))?;
    println!("{json}");
    Ok(())
}

pub fn pack(file_name: Option<String>, project_manifest: ProjectManifest) -> Result<(), String> {
    let deployment_path = get_default_deployment_path(&project_manifest, &StacksNetwork::Devnet)
        .map_err(|e| format!("failed to get default deployment path: {e}"))?;

    let deployment_manifest = DeploymentSpecification::from_config_file(
        &deployment_path,
        &project_manifest
            .location
            .get_project_root_location()
            .map_err(|e| format!("failed to get project root location: {e}"))?,
    )
    .map_err(|e| format!("failed to create deployment plan: {e}"))?;

    let network_manifest = NetworkManifest::from_project_manifest_location(
        &project_manifest.location,
        &StacksNetwork::Devnet.get_networks(),
        None,
        None,
    )
    .map_err(|e| format!("failed to get project manifest: {e}"))?;

    let package = ConfigurationPackage {
        deployment_plan: deployment_manifest,
        network_manifest,
        project_manifest,
    };

    match file_name {
        Some(name) => pack_to_file(&name, package),
        None => pack_to_stdout(package),
    }
}
