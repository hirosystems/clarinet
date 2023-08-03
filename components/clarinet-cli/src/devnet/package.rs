use std::fs::{read_to_string, File};
use std::io::{self, Write};
use std::process;

use clarinet_deployments::types::DeploymentSpecificationFile;
use clarinet_files::NetworkManifestFile;

use toml::value::Value;

#[derive(Serialize, Deserialize, Debug)]
struct Project {
    name: String,
    description: Option<String>,
    authors: Vec<String>,
    telemetry: Option<bool>,
    cache_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClarinetSpecificationFile {
    project: Project,
    contracts: Option<Value>,
    repl: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfigurationPackage {
    deployment_plan: DeploymentSpecificationFile,
    devnet_config: NetworkManifestFile,
    clarinet_config: ClarinetSpecificationFile,
}

fn get_devnet_config() -> NetworkManifestFile {
    let file_content = match read_to_string("./settings/Devnet.toml") {
        Ok(content) => content,
        Err(err) => {
            println!("Could not read Devnet.toml file: {}", err);
            process::exit(1);
        }
    };

    let devnet_config: NetworkManifestFile = match toml::from_str(&file_content) {
        Ok(data) => data,
        Err(err) => {
            println!("Unable to load data from Devnet.toml file: {}", err);
            process::exit(1);
        }
    };

    devnet_config
}

fn get_clarinet_config() -> ClarinetSpecificationFile {
    let clarinet_config_content = match read_to_string("./Clarinet.toml") {
        Ok(content) => content,
        Err(e) => {
            println!("Could not read Clarinet.toml file: {}", e);
            process::exit(1);
        }
    };

    let clarinet_config: ClarinetSpecificationFile = match toml::from_str(&clarinet_config_content)
    {
        Ok(data) => data,
        Err(err) => {
            println!("Unable to load data from Clarinet.toml file: {}", err);
            process::exit(1);
        }
    };

    clarinet_config
}

fn get_deployment_plan() -> DeploymentSpecificationFile {
    let deployment_spec_file = File::open("./deployments/default.simnet-plan.yaml").unwrap();
    let deployment_plan: DeploymentSpecificationFile =
        serde_yaml::from_reader(deployment_spec_file).unwrap();

    deployment_plan
}

fn pack_to_file(file_name: &str) -> Result<(), io::Error> {
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

    let package = ConfigurationPackage {
        deployment_plan: get_deployment_plan(),
        devnet_config: get_devnet_config(),
        clarinet_config: get_clarinet_config(),
    };

    match serde_json::to_writer(file, &package) {
        Ok(_) => println!("{} file generated with success", file_name),
        Err(e) => println!("Unable to generate the json file: {}", e),
    };

    Ok(())
}

fn pack_to_stdout() {
    let package = ConfigurationPackage {
        deployment_plan: get_deployment_plan(),
        devnet_config: get_devnet_config(),
        clarinet_config: get_clarinet_config(),
    };

    let s = serde_json::to_string(&package).unwrap();
    io::stdout().write(s.as_bytes()).ok();
}

pub fn pack(file_name: Option<String>) -> Result<(), io::Error> {
    match file_name {
        Some(name) => pack_to_file(&name)?,
        None => pack_to_stdout(),
    }

    Ok(())
}
