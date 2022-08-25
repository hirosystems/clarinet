use clarinet_files::{FileAccessor, FileLocation};
use clarity_repl::clarity::types::QualifiedContractIdentifier;
use reqwest;

pub async fn retrieve_contract(
    contract_id: &QualifiedContractIdentifier,
    cache_location: &FileLocation,
    file_accessor: &Option<&Box<dyn FileAccessor>>,
) -> Result<(String, FileLocation), String> {
    let contract_deployer = contract_id.issuer.to_address();
    let contract_name = contract_id.name.to_string();

    let mut contract_location = cache_location.clone();
    contract_location.append_path(&format!("{}.{}.clar", contract_deployer, contract_name))?;

    let contract_source = match file_accessor {
        None => contract_location.read_content_as_utf8(),
        Some(file_accessor) => {
            match file_accessor
                .read_contract_content(contract_location.clone())
                .await
            {
                Ok((_, source)) => Ok(source),
                Err(err) => Err(err),
            }
        }
    };

    if contract_source.is_ok() {
        return Ok((contract_source.unwrap(), contract_location));
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

    let code = fetch_contract(request_url).await?.source;

    let result = match file_accessor {
        None => contract_location.write_content(code.as_bytes()),
        Some(file_accessor) => {
            file_accessor
                .write_file(contract_location.clone(), code.as_bytes())
                .await
        }
    };

    match result {
        Ok(_) => Ok((code, contract_location)),
        Err(err) => Err(err),
    }
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
