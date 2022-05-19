use clarity_repl::clarity::types::QualifiedContractIdentifier;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

pub fn retrieve_contract(
    contract_id: &QualifiedContractIdentifier,
    use_cache: bool,
    cache_path: Option<PathBuf>,
) -> Result<(String, PathBuf), String> {
    let contract_deployer = contract_id.issuer.to_address();
    let contract_name = contract_id.name.to_string();
    let mut file_path = PathBuf::new();

    if use_cache {
        if let Some(ref cache_path) = cache_path {
            let mut path = PathBuf::from(cache_path);
            path.push(format!("{}.clar", contract_id));
            if let Ok(data) = fs::read_to_string(&path) {
                return Ok((data, path));
            }
        }
    }

    let stacks_node_addr = if contract_deployer.starts_with("SP") {
        "https://stacks-node-api.mainnet.stacks.co".to_string()
    } else {
        "https://stacks-node-api.testnet.stacks.co".to_string()
    };

    let request_url = format!(
        "{host}/v2/contracts/source/{addr}/{name}?proof=0",
        host = stacks_node_addr,
        addr = contract_deployer,
        name = contract_name
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = rt.block_on(async { fetch_contract(request_url).await });
    let code = response.source.to_string();

    if use_cache {
        if let Some(ref cache_path) = cache_path {
            file_path = PathBuf::from(cache_path);
            let _ = fs::create_dir_all(&file_path);
            file_path.push(format!("{}.clar", contract_id));

            if let Ok(ref mut file) = File::create(&file_path) {
                let _ = file.write_all(code.as_bytes());
            }
        }
    }

    Ok((code, file_path))
}

#[derive(Deserialize, Debug, Default, Clone)]
struct Contract {
    source: String,
    publish_height: u32,
}

async fn fetch_contract(request_url: String) -> Contract {
    let response: Contract = reqwest::get(&request_url)
        .await
        .expect("Unable to retrieve contract")
        .json()
        .await
        .expect("Unable to parse contract");
    return response;
}
