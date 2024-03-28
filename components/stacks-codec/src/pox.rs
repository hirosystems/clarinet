use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct PoxInfo {
    pub contract_id: String,
    pub pox_activation_threshold_ustx: u64,
    pub first_burnchain_block_height: u32,
    pub current_burnchain_block_height: u32,
    pub prepare_phase_block_length: u32,
    pub reward_phase_block_length: u32,
    pub reward_slots: u32,
    pub reward_cycle_id: u32,
    pub reward_cycle_length: u32,
    pub total_liquid_supply_ustx: u64,
    pub current_cycle: CurrentPoxCycle,
    pub next_cycle: NextPoxCycle,
}

impl PoxInfo {
    pub fn mainnet_default() -> PoxInfo {
        PoxInfo {
            contract_id: "SP000000000000000000002Q6VF78.pox-3".into(),
            pox_activation_threshold_ustx: 0,
            first_burnchain_block_height: 666050,
            prepare_phase_block_length: 100,
            reward_phase_block_length: 2000,
            reward_slots: 4000,
            total_liquid_supply_ustx: 1368787887756275,
            ..Default::default()
        }
    }

    pub fn testnet_default() -> PoxInfo {
        PoxInfo {
            contract_id: "ST000000000000000000002AMW42H.pox-3".into(),
            pox_activation_threshold_ustx: 0,
            current_burnchain_block_height: 2000000,
            first_burnchain_block_height: 2000000,
            prepare_phase_block_length: 50,
            reward_phase_block_length: 1000,
            reward_slots: 2000,
            total_liquid_supply_ustx: 41412139686144074,
            ..Default::default()
        }
    }

    pub fn devnet_default() -> PoxInfo {
        Self::default()
    }
}

impl Default for PoxInfo {
    fn default() -> PoxInfo {
        PoxInfo {
            contract_id: "ST000000000000000000002AMW42H.pox".into(),
            pox_activation_threshold_ustx: 0,
            current_burnchain_block_height: 100,
            first_burnchain_block_height: 100,
            prepare_phase_block_length: 4,
            reward_phase_block_length: 6,
            reward_cycle_length: 10,
            reward_slots: 10,
            total_liquid_supply_ustx: 1000000000000000,
            reward_cycle_id: 0,
            current_cycle: CurrentPoxCycle::default(),
            next_cycle: NextPoxCycle::default(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct CurrentPoxCycle {
    pub id: u64,
    pub min_threshold_ustx: u64,
    pub stacked_ustx: u64,
    pub is_pox_active: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct NextPoxCycle {
    pub min_threshold_ustx: u64,
    pub stacked_ustx: u64,
    pub blocks_until_prepare_phase: i16,
    pub blocks_until_reward_phase: i16,
}
