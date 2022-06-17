use clarinet_files::FileLocation;
use clarity_repl::clarity::types::QualifiedContractIdentifier;
use reqwest;

pub async fn retrieve_contract(
    contract_id: &QualifiedContractIdentifier,
    cache_location: &FileLocation,
) -> Result<(String, FileLocation), String> {
    let contract_deployer = contract_id.issuer.to_address();
    let contract_name = contract_id.name.to_string();

    let mut contract_location = cache_location.clone();
    contract_location
        .append_relative_path(&format!("{}.{}.clar", contract_deployer, contract_name))?;

    if let Ok(contract_source) = contract_location.read_content_as_utf8() {
        return Ok((contract_source, contract_location));
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

    let response = fetch_contract(request_url).await?;
    let code = response.source.to_string();
    contract_location.write_content(code.as_bytes());

    Ok((code, contract_location))
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Default, Clone)]
struct Contract {
    source: String,
    publish_height: u32,
}

async fn fetch_contract(request_url: String) -> Result<Contract, String> {
    let response = reqwest::get(&request_url)
        .await
        .map_err(|_| format!("Unable to retrieve contract {}", request_url))?;

    let contract = response
        .json()
        .await
        .map_err(|_| format!("Unable to parse contract {}", request_url))?;

    Ok(contract)
}
