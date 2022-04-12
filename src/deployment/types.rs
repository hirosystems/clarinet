use crate::types::StacksNetwork;

pub struct ContractPublishAction {
    name: String,
    sender: String,
    path: String,
    version: u16,
    hash: String,
}

pub struct ContractCallAction {
    name: String,
    sender: String
}

pub struct StacksHookUpdateAction {

}

pub struct BitcoinHookUpdateAction {

}

pub enum DeploymentAction {
    ContractPublish(ContractPublishAction),
    ContractCall(ContractCallAction),
    StacksHook(StacksHookUpdateAction),
    BitcoinHook(BitcoinHookUpdateAction),
}

pub struct DeploymentPlan {
    pub network: StacksNetwork,
    pub actions: Vec<DeploymentAction>,
}

pub struct DeploymentPlanFile {

}

pub struct DeploymentState {
    pub network: StacksNetwork,
    pub actions: Vec<DeploymentAction>,
}

pub struct DeploymentStateFile {

}