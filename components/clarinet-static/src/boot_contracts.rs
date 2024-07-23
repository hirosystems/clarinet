use std::collections::BTreeMap;

use clarinet_core::{ClarityCodeSource, ClarityContract, ContractDeployer};
use clarity::{
    types::StacksEpochId,
    vm::{
        ast::{build_ast_with_diagnostics, ContractAST},
        types::{PrincipalData, QualifiedContractIdentifier, StandardPrincipalData},
        ClarityVersion,
    },
};

use super::boot::{STACKS_BOOT_CODE_MAINNET, STACKS_BOOT_CODE_TESTNET};

pub static BOOT_TESTNET_ADDRESS: &str = "ST000000000000000000002AMW42H";
pub static BOOT_MAINNET_ADDRESS: &str = "SP000000000000000000002Q6VF78";

pub static V1_BOOT_CONTRACTS: &[&str] = &["bns"];
pub static V2_BOOT_CONTRACTS: &[&str] = &["pox-2", "costs-3"];
pub static V3_BOOT_CONTRACTS: &[&str] = &["pox-3"];
pub static V4_BOOT_CONTRACTS: &[&str] = &["pox-4"];

lazy_static! {
    pub static ref BOOT_TESTNET_PRINCIPAL: StandardPrincipalData =
        PrincipalData::parse_standard_principal(BOOT_TESTNET_ADDRESS).unwrap();
    pub static ref BOOT_MAINNET_PRINCIPAL: StandardPrincipalData =
        PrincipalData::parse_standard_principal(BOOT_MAINNET_ADDRESS).unwrap();
    pub static ref BOOT_CONTRACTS_DATA: BTreeMap<QualifiedContractIdentifier, (ClarityContract, ContractAST)> = {
        let mut result = BTreeMap::new();

        let deploy: [(&StandardPrincipalData, [(&str, &str); 13]); 2] = [
            (&*BOOT_TESTNET_PRINCIPAL, *STACKS_BOOT_CODE_TESTNET),
            (&*BOOT_MAINNET_PRINCIPAL, *STACKS_BOOT_CODE_MAINNET),
        ];

        // let interpreter =
        //     ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        for (deployer, boot_code) in deploy.iter() {
            for (name, code) in boot_code.iter() {
                let (epoch, clarity_version) = match *name {
                    "pox-4" | "signers" | "signers-voting" => {
                        (StacksEpochId::Epoch25, ClarityVersion::Clarity2)
                    }
                    "pox-3" => (StacksEpochId::Epoch24, ClarityVersion::Clarity2),
                    "pox-2" | "costs-3" => (StacksEpochId::Epoch21, ClarityVersion::Clarity2),
                    "cost-2" => (StacksEpochId::Epoch2_05, ClarityVersion::Clarity1),
                    _ => (StacksEpochId::Epoch20, ClarityVersion::Clarity1),
                };

                let boot_contract = ClarityContract {
                    code_source: ClarityCodeSource::ContractInMemory(code.to_string()),
                    deployer: ContractDeployer::Address(deployer.to_address()),
                    name: name.to_string(),
                    epoch,
                    clarity_version,
                };

                let contract_id =boot_contract.expect_resolved_contract_identifier(None);
                let (ast, _, _) = build_ast_with_diagnostics(&contract_id, code, &mut (), clarity_version, epoch);

                result.insert(
                    boot_contract.expect_resolved_contract_identifier(None),
                    (boot_contract, ast),
                );
            }
        }
        result
    };
}
