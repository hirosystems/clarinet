use std::fs::File;
use std::io::{self, Write};
use std::process;
use serde::{Serialize, Deserialize};

use clarinet_deployments::get_default_deployment_path;
use clarinet_deployments::types::DeploymentSpecification;
use clarinet_files::chainhook_types::StacksNetwork;
use clarinet_files::{NetworkManifest, ProjectManifest};

#[derive(Serialize, Deserialize, Debug)]
struct ConfigurationPackage {
    deployment_plan: DeploymentSpecification,
    devnet_config: NetworkManifest,
    clarinet_config: ProjectManifest,
}

fn pack_to_file(file_name: &str, package: ConfigurationPackage) -> Result<(), io::Error> {
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

    match serde_json::to_writer(file, &package) {
        Ok(_) => println!("{} file generated with success", file_name),
        Err(e) => println!("Unable to generate the json file: {}", e),
    };

    Ok(())
}

fn pack_to_stdout(package: ConfigurationPackage) {
    let json = serde_json::to_value(package).unwrap();
    io::stdout().write(json.to_string().as_bytes()).ok();
}

pub fn pack(file_name: Option<String>, project_manifest: ProjectManifest) -> Result<(), io::Error> {
    let deployment_path =
        get_default_deployment_path(&project_manifest, &StacksNetwork::Devnet).unwrap();

    let deployment_manifest = DeploymentSpecification::from_config_file(
        &deployment_path,
        &project_manifest
            .location
            .get_project_root_location()
            .unwrap(),
    )
    .unwrap();

    let network_manifest = NetworkManifest::from_project_manifest_location(
        &project_manifest.location,
        &StacksNetwork::Devnet.get_networks(),
        None,
        None,
    )
    .unwrap();

    let package = ConfigurationPackage {
        deployment_plan: deployment_manifest,
        devnet_config: network_manifest,
        clarinet_config: project_manifest,
    };

    match file_name {
        Some(name) => pack_to_file(&name, package)?,
        None => pack_to_stdout(package),
    }

    Ok(())
}