use crate::deployment::{ContractStatus, ContractUpdate};
use tui::widgets::ListState;

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

pub struct App<'a> {
    pub node_url: &'a str,
    pub contracts: StatefulList<ContractUpdate>,
}

impl<'a> App<'a> {
    pub fn new(node_url: &'a str, contracts: Vec<(String, String)>) -> App<'a> {
        let tracked_contracts = contracts
            .iter()
            .map(|(deployer, name)| ContractUpdate {
                status: ContractStatus::Queued,
                contract_id: format!("{}.{}", deployer, name),
                comment: None,
            })
            .collect::<Vec<_>>();

        App {
            node_url,
            contracts: StatefulList {
                state: ListState::default(),
                items: tracked_contracts,
            },
        }
    }

    pub fn on_tick(&mut self) {}

    pub fn reset(&mut self) {}

    pub fn display_contract_status_update(&mut self, update: ContractUpdate) {
        let index_found = self
            .contracts
            .items
            .iter()
            .position(|contract| contract.contract_id == update.contract_id);
        if let Some(index) = index_found {
            self.contracts.items.remove(index);
            self.contracts.items.insert(index, update);
        }
    }
}
