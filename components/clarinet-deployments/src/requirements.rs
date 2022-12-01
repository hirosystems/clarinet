use clarinet_files::{FileAccessor, FileLocation};
use clarity_repl::clarity::{vm::types::QualifiedContractIdentifier, ClarityVersion};
use reqwest;

pub async fn retrieve_contract(
    contract_id: &QualifiedContractIdentifier,
    cache_location: &FileLocation,
    file_accessor: &Option<&Box<dyn FileAccessor>>,
) -> Result<(String, ClarityVersion, FileLocation), String> {
    let contract_deployer = contract_id.issuer.to_address();
    let contract_name = contract_id.name.to_string();

    let mut contract_location = cache_location.clone();
    contract_location.append_path("requirements")?;
    contract_location.append_path(&format!("{}.{}.clar", contract_deployer, contract_name))?;

    let contract_source = match file_accessor {
        None => contract_location.read_content_as_utf8(),
        Some(file_accessor) => file_accessor.read_file(contract_location.to_string()).await,
    };

    if contract_source.is_ok() {
        return Ok((
            contract_source.unwrap(),
            ClarityVersion::Clarity1,
            contract_location,
        ));
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

    let contract = fetch_contract(request_url).await?;

    let result = match file_accessor {
        None => contract_location.write_content(contract.source.as_bytes()),
        Some(file_accessor) => {
            file_accessor
                .write_file(contract_location.to_string(), contract.source.as_bytes())
                .await
        }
    };

    let clarity_version = {
        let version = contract.clarity_version.unwrap_or(1);
        if version.eq(&1) {
            ClarityVersion::Clarity1
        } else if version.eq(&2) {
            ClarityVersion::Clarity2
        } else {
            return Err(format!(
                "unable to parse clarity_version (can either be '1' or '2'",
            ));
        }
    };

    match result {
        Ok(_) => Ok((contract.source, clarity_version, contract_location)),
        Err(err) => Err(err),
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Default, Clone)]
struct Contract {
    source: String,
    publish_height: u32,
    clarity_version: Option<u8>,
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
