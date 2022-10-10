pub mod types;
mod ui;






pub use ui::start_ui;

use hiro_system_kit;

use clarinet_deployments::types::{
    DeploymentGenerationArtifacts, DeploymentSpecification,
};

use clarinet_files::{FileLocation, ProjectManifest};









use chainhook_types::StacksNetwork;



use serde_yaml;

use std::fs::{self};

use std::path::PathBuf;


#[derive(Deserialize, Debug)]
pub struct Balance {
    pub balance: String,
    pub nonce: u64,
    pub balance_proof: String,
    pub nonce_proof: String,
}

pub fn get_absolute_deployment_path(
    manifest: &ProjectManifest,
    relative_deployment_path: &str,
) -> Result<FileLocation, String> {
    let mut deployment_path = manifest.location.get_project_root_location()?;
    deployment_path.append_path(relative_deployment_path)?;
    Ok(deployment_path)
}

pub fn get_default_deployment_path(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
) -> Result<FileLocation, String> {
    let mut deployment_path = manifest.location.get_project_root_location()?;
    deployment_path.append_path("deployments")?;
    deployment_path.append_path(match network {
        StacksNetwork::Simnet => "default.simnet-plan.yaml",
        StacksNetwork::Devnet => "default.devnet-plan.yaml",
        StacksNetwork::Testnet => "default.testnet-plan.yaml",
        StacksNetwork::Mainnet => "default.mainnet-plan.yaml",
    })?;
    Ok(deployment_path)
}

pub fn generate_default_deployment(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
    _no_batch: bool,
) -> Result<(DeploymentSpecification, DeploymentGenerationArtifacts), String> {
    let future = clarinet_deployments::generate_default_deployment(manifest, network, false, None);
    hiro_system_kit::nestable_block_on(future)
}

#[allow(dead_code)]
pub fn read_deployment_or_generate_default(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
) -> Result<
    (
        DeploymentSpecification,
        Option<DeploymentGenerationArtifacts>,
    ),
    String,
> {
    let default_deployment_file_path = get_default_deployment_path(&manifest, network)?;
    let (deployment, artifacts) = if default_deployment_file_path.exists() {
        (
            load_deployment(manifest, &default_deployment_file_path)?,
            None,
        )
    } else {
        let (deployment, artifacts) = generate_default_deployment(manifest, network, false)?;
        (deployment, Some(artifacts))
    };
    Ok((deployment, artifacts))
}

pub fn check_deployments(manifest: &ProjectManifest) -> Result<(), String> {
    let project_root_location = manifest.location.get_project_root_location()?;
    let files = get_deployments_files(&project_root_location)?;
    for (path, relative_path) in files.into_iter() {
        let _spec = match DeploymentSpecification::from_config_file(
            &FileLocation::from_path(path),
            &project_root_location,
        ) {
            Ok(spec) => spec,
            Err(msg) => {
                println!("{} {} syntax incorrect\n{}", red!("x"), relative_path, msg);
                continue;
            }
        };
        println!("{} {} succesfully checked", green!("✔"), relative_path);
    }
    Ok(())
}

pub fn load_deployment(
    manifest: &ProjectManifest,
    deployment_plan_location: &FileLocation,
) -> Result<DeploymentSpecification, String> {
    let project_root_location = manifest.location.get_project_root_location()?;
    let spec = match DeploymentSpecification::from_config_file(
        &deployment_plan_location,
        &project_root_location,
    ) {
        Ok(spec) => spec,
        Err(msg) => {
            return Err(format!(
                "{} {} syntax incorrect\n{}",
                red!("x"),
                deployment_plan_location.to_string(),
                msg
            ));
        }
    };
    Ok(spec)
}

fn get_deployments_files(
    project_root_location: &FileLocation,
) -> Result<Vec<(PathBuf, String)>, String> {
    let mut project_dir = project_root_location.clone();
    let prefix_len = project_dir.to_string().len() + 1;
    project_dir.append_path("deployments")?;
    let paths = match fs::read_dir(&project_dir.to_string()) {
        Ok(paths) => paths,
        Err(_) => return Ok(vec![]),
    };
    let mut plans_paths = vec![];
    for path in paths {
        let file = path.unwrap().path();
        let is_extension_valid = file
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| Some(ext == "yml" || ext == "yaml"));

        if let Some(true) = is_extension_valid {
            let relative_path = file.clone();
            let (_, relative_path) = relative_path.to_str().unwrap().split_at(prefix_len);
            plans_paths.push((file, relative_path.to_string()));
        }
    }

    Ok(plans_paths)
}

pub fn write_deployment(
    deployment: &DeploymentSpecification,
    target_location: &FileLocation,
    prompt_override: bool,
) -> Result<(), String> {
    if target_location.exists() && prompt_override {
        println!(
            "Deployment {} already exists.\n{}?",
            target_location.to_string(),
            yellow!("Overwrite [Y/n]")
        );
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();
        if buffer.starts_with("n") {
            return Err(format!("deployment update aborted"));
        }
    }

    let file = deployment.to_specification_file();

    let content = match serde_yaml::to_string(&file) {
        Ok(res) => res,
        Err(err) => return Err(format!("failed serializing deployment\n{}", err)),
    };

    target_location.write_content(content.as_bytes())?;
    Ok(())
}
