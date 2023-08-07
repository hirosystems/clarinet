use std::fs::{read_to_string, File};
use std::io::{self, Write};
use std::process;

use clarinet_deployments::types::DeploymentSpecificationFile;
use clarinet_files::{FileLocation, NetworkManifestFile, ProjectManifestFile};

#[derive(Serialize, Deserialize, Debug)]
struct ConfigurationPackage {
    deployment_plan: DeploymentSpecificationFile,
    devnet_config: NetworkManifestFile,
    clarinet_config: ProjectManifestFile,
}

fn get_devnet_config() -> Result<NetworkManifestFile, io::Error> {
    let file_content = read_to_string("./settings/Devnet.toml")?;

    let devnet_config: NetworkManifestFile = match toml::from_str(&file_content) {
        Ok(data) => data,
        Err(err) => {
            println!("Unable to load data from Devnet.toml file: {}", err);
            process::exit(1);
        }
    };

    Ok(devnet_config)
}

fn get_clarinet_config(manifest_location: FileLocation) -> Result<ProjectManifestFile, io::Error> {
    let clarinet_config_content = read_to_string(manifest_location.to_string())?;

    let clarinet_config: ProjectManifestFile = match toml::from_str(&clarinet_config_content) {
        Ok(data) => data,
        Err(err) => {
            println!("Unable to load data from Clarinet.toml file: {}", err);
            process::exit(1);
        }
    };

    Ok(clarinet_config)
}

fn get_deployment_plan() -> Result<DeploymentSpecificationFile, io::Error> {
    let deployment_spec_file = File::open("./deployments/default.simnet-plan.yaml")?;
    let deployment_plan: DeploymentSpecificationFile =
        serde_yaml::from_reader(deployment_spec_file).unwrap();

    Ok(deployment_plan)
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
    let s = serde_json::to_string(&package).unwrap();
    io::stdout().write(s.as_bytes()).ok();
}

pub fn pack(file_name: Option<String>, manifest_location: FileLocation) -> Result<(), io::Error> {
    let package = ConfigurationPackage {
        deployment_plan: get_deployment_plan()?,
        devnet_config: get_devnet_config()?,
        clarinet_config: get_clarinet_config(manifest_location)?,
    };

    match file_name {
        Some(name) => pack_to_file(&name, package)?,
        None => pack_to_stdout(package),
    }

    Ok(())
}
