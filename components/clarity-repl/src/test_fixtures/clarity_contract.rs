use clarity::{types::StacksEpochId, vm::ClarityVersion};

use crate::repl::{
    ClarityCodeSource, ClarityContract, ContractDeployer, DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH,
};

impl ClarityContract {
    pub fn fixture() -> Self {
        let snippet = [
            "(define-data-var x uint u0)",
            "(define-read-only (get-x)",
            "  (var-get x))",
            "(define-public (incr)",
            "  (let ((new-x (+ (var-get x) u1)))",
            "    (var-set x new-x)",
            "    (ok new-x)))",
        ]
        .join("\n");

        Self {
            code_source: ClarityCodeSource::ContractInMemory(snippet.to_string()),
            name: "contract".into(),
            deployer: ContractDeployer::DefaultDeployer,
            clarity_version: DEFAULT_CLARITY_VERSION,
            epoch: DEFAULT_EPOCH,
        }
    }
}

pub struct ClarityContractBuilder {
    contract: ClarityContract,
}

impl Default for ClarityContractBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClarityContractBuilder {
    pub fn new() -> Self {
        Self {
            contract: ClarityContract::fixture(),
        }
    }

    pub fn code_source(mut self, code_source: String) -> Self {
        self.contract.code_source = ClarityCodeSource::ContractInMemory(code_source);
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        name.clone_into(&mut self.contract.name);
        self
    }

    pub fn deployer(mut self, address: &str) -> Self {
        self.contract.deployer = ContractDeployer::Address(address.to_owned());
        self
    }

    pub fn clarity_version(mut self, clarity_version: ClarityVersion) -> Self {
        self.contract.clarity_version = clarity_version;
        self
    }

    pub fn epoch(mut self, epoch: StacksEpochId) -> Self {
        self.contract.epoch = epoch;
        self
    }

    pub fn build(self) -> ClarityContract {
        let default_version = ClarityVersion::default_for_epoch(self.contract.epoch);
        let clarity_version = self.contract.clarity_version;

        assert!(
            !(clarity_version > default_version),
            "invalid clarity version {} for epoch {}",
            clarity_version,
            self.contract.epoch
        );

        self.contract
    }
}
