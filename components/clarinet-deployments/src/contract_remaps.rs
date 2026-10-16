//! Requirements that already exist on the target network are pointed at that
//! deployment instead of being redeployed under the project's deployer.

use std::collections::HashMap;

use clarinet_files::{ProjectManifest, StacksNetwork};
use clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::repl::boot::{SBTC_BOOT_CONTRACTS, SBTC_TESTNET_ADDRESS_PRINCIPAL};

use crate::requirements::fetch_published_source;
use crate::source_remap::remap_contract_ids;
use crate::types::ContractRemaps;

/// Well-known contracts and where they are deployed on `network`.
///
/// Applied unless the project sets `default_contract_remaps = false`; a
/// requirement's own `testnet` / `mainnet` entry takes precedence. Only list
/// contracts whose deployment on that network is maintained, since every
/// project that references them is pointed there.
fn default_contract_remaps(network: &StacksNetwork) -> ContractRemaps {
    match network {
        StacksNetwork::Testnet => SBTC_BOOT_CONTRACTS
            .iter()
            .map(|(mainnet_id, _)| {
                let testnet_id = QualifiedContractIdentifier::new(
                    SBTC_TESTNET_ADDRESS_PRINCIPAL.clone(),
                    mainnet_id.name.clone(),
                );
                (mainnet_id.clone(), testnet_id)
            })
            .collect(),
        StacksNetwork::Simnet | StacksNetwork::Devnet | StacksNetwork::Mainnet => {
            ContractRemaps::new()
        }
    }
}

fn parse_contract_id(contract_id: &str) -> Result<QualifiedContractIdentifier, String> {
    QualifiedContractIdentifier::parse(contract_id)
        .map_err(|_| format!("malformatted contract_id: {contract_id}"))
}

/// The requirement remaps the project itself declares for `network`.
pub fn requirement_remaps(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
) -> Result<ContractRemaps, String> {
    manifest
        .project
        .requirements
        .iter()
        .flatten()
        .filter_map(|requirement| {
            let remap = requirement.remap_for(network)?;
            Some((&requirement.contract_id, remap))
        })
        .map(|(contract_id, remap)| {
            Ok((parse_contract_id(contract_id)?, parse_contract_id(remap)?))
        })
        .collect()
}

/// Every remap that applies on `network`: the defaults, then the project's
/// own on top.
pub fn resolve_contract_remaps(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
) -> Result<ContractRemaps, String> {
    let mut remaps = if manifest.project.default_contract_remaps {
        default_contract_remaps(network)
    } else {
        ContractRemaps::new()
    };
    remaps.extend(requirement_remaps(manifest, network)?);
    Ok(remaps)
}

/// Warnings for project remaps whose destination is not published, or whose
/// published source differs from the contract it stands in for.
///
/// The canonical source is compared after applying `all_remaps`, so a copy
/// that only differs by pointing at the remapped contracts still matches.
/// Best effort: a network failure becomes a warning too, so an offline plan
/// generation still succeeds.
pub async fn check_requirement_remaps(
    remaps: &ContractRemaps,
    sources: &HashMap<QualifiedContractIdentifier, String>,
    all_remaps: &ContractRemaps,
    api_base_url: Option<&str>,
) -> Vec<String> {
    let normalize = |source: &str| {
        remap_contract_ids(
            source,
            all_remaps
                .iter()
                .map(|(src, dst)| (src.to_string(), dst.to_string())),
        )
    };
    let mut warnings = vec![];
    for (contract_id, remap) in remaps {
        match fetch_published_source(remap, api_base_url).await {
            Ok(None) => warnings.push(format!(
                "requirement {contract_id} is remapped to {remap}, which is not published"
            )),
            Ok(Some(remap_source)) => {
                if sources
                    .get(contract_id)
                    .is_some_and(|source| normalize(source) != remap_source)
                {
                    warnings.push(format!("requirement {contract_id} is remapped to {remap}, whose source differs; check that it provides the same interface"));
                }
            }
            Err(e) => warnings.push(format!(
                "unable to verify remap of {contract_id} to {remap}: {e}"
            )),
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use clarity_repl::repl::boot::{SBTC_MAINNET_ADDRESS, SBTC_TESTNET_ADDRESS};

    use super::*;

    #[test]
    fn sbtc_is_remapped_to_its_testnet_deployment() {
        let remaps = default_contract_remaps(&StacksNetwork::Testnet);
        for name in ["sbtc-registry", "sbtc-token", "sbtc-deposit"] {
            let mainnet_id = parse_contract_id(&format!("{SBTC_MAINNET_ADDRESS}.{name}")).unwrap();
            assert_eq!(
                remaps[&mainnet_id].to_string(),
                format!("{SBTC_TESTNET_ADDRESS}.{name}")
            );
        }
    }

    #[test]
    fn defaults_only_apply_to_testnet() {
        for network in [
            StacksNetwork::Simnet,
            StacksNetwork::Devnet,
            StacksNetwork::Mainnet,
        ] {
            assert!(default_contract_remaps(&network).is_empty(), "{network:?}");
        }
    }

    #[tokio::test]
    async fn unpublished_or_different_destinations_are_reported() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock(
                "GET",
                "/extended/v1/contract/ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.missing",
            )
            .with_status(404)
            .create_async()
            .await;
        server
            .mock(
                "GET",
                "/extended/v1/contract/ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.other",
            )
            .with_status(200)
            .with_body(r#"{"source_code": "(define-read-only (b) u2)", "block_height": 1}"#)
            .create_async()
            .await;
        server
            .mock(
                "GET",
                "/extended/v1/contract/ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.same",
            )
            .with_status(200)
            .with_body(r#"{"source_code": "(define-read-only (a) u1)", "block_height": 1}"#)
            .create_async()
            .await;

        let id = |id: &str| parse_contract_id(id).unwrap();
        let remaps = ContractRemaps::from([
            (
                id("SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.missing"),
                id("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.missing"),
            ),
            (
                id("SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.other"),
                id("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.other"),
            ),
            (
                id("SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.same"),
                id("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.same"),
            ),
        ]);
        let sources = HashMap::from([
            (
                id("SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.other"),
                "(define-read-only (a) u1)".to_string(),
            ),
            (
                id("SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.same"),
                "(define-read-only (a) u1)".to_string(),
            ),
        ]);

        let warnings =
            check_requirement_remaps(&remaps, &sources, &remaps, Some(&server.url())).await;

        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(
            warnings[0].ends_with(".missing, which is not published"),
            "{warnings:?}"
        );
        assert!(
            warnings[1].contains(".other, whose source differs"),
            "{warnings:?}"
        );
    }

    #[tokio::test]
    async fn a_copy_pointing_at_remapped_contracts_matches() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock(
                "GET",
                "/extended/v1/contract/ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.wrapper",
            )
            .with_status(200)
            .with_body(
                serde_json::json!({
                    "source_code": "(contract-call? 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.base f)",
                    "block_height": 1
                })
                .to_string(),
            )
            .create_async()
            .await;

        let id = |id: &str| parse_contract_id(id).unwrap();
        let wrapper = id("SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.wrapper");
        let wrapper_remap = id("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.wrapper");
        let remaps = ContractRemaps::from([(wrapper.clone(), wrapper_remap.clone())]);
        let all_remaps = ContractRemaps::from([
            (wrapper.clone(), wrapper_remap),
            (
                id("SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.base"),
                id("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.base"),
            ),
        ]);
        let sources = HashMap::from([(
            wrapper,
            "(contract-call? 'SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.base f)".to_string(),
        )]);

        let warnings =
            check_requirement_remaps(&remaps, &sources, &all_remaps, Some(&server.url())).await;

        assert_eq!(warnings, Vec::<String>::new());
    }
}
