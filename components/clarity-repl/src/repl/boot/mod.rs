// Copyright (C) 2013-2020 Blockstack PBC, a public benefit corporation
// Copyright (C) 2020 Stacks Open Internet Foundation
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

// This code is copied from stacks-blockchain/src/chainstate/atacks/boot/mod.rs

const BOOT_CODE_POX_BODY: &'static str = std::include_str!("pox.clar");
const BOOT_CODE_POX_TESTNET_CONSTS: &'static str = std::include_str!("pox-testnet.clar");
const BOOT_CODE_POX_MAINNET_CONSTS: &'static str = std::include_str!("pox-mainnet.clar");
const BOOT_CODE_LOCKUP: &'static str = std::include_str!("lockup.clar");
pub const BOOT_CODE_COSTS: &'static str = std::include_str!("costs.clar");
pub const BOOT_CODE_COSTS_2: &'static str = std::include_str!("costs-2.clar");
pub const BOOT_CODE_COSTS_2_TESTNET: &'static str = std::include_str!("costs-2-testnet.clar");
const BOOT_CODE_COST_VOTING_MAINNET: &'static str = std::include_str!("cost-voting.clar");
const BOOT_CODE_BNS: &'static str = std::include_str!("bns.clar");
const BOOT_CODE_GENESIS: &'static str = std::include_str!("genesis.clar");
pub const POX_1_NAME: &'static str = "pox";
pub const POX_2_NAME: &'static str = "pox-2";

const POX_2_TESTNET_CONSTS: &'static str = std::include_str!("pox-testnet.clar");
const POX_2_MAINNET_CONSTS: &'static str = std::include_str!("pox-mainnet.clar");
const POX_2_BODY: &'static str = std::include_str!("pox-2.clar");

pub const COSTS_1_NAME: &'static str = "costs";
pub const COSTS_2_NAME: &'static str = "costs-2";

lazy_static! {
    pub static ref BOOT_CODE_POX_MAINNET: String =
        format!("{}\n{}", BOOT_CODE_POX_MAINNET_CONSTS, BOOT_CODE_POX_BODY);
    pub static ref BOOT_CODE_POX_TESTNET: String =
        format!("{}\n{}", BOOT_CODE_POX_TESTNET_CONSTS, BOOT_CODE_POX_BODY);
    pub static ref POX_2_MAINNET_CODE: String =
        format!("{}\n{}", BOOT_CODE_POX_MAINNET_CONSTS, POX_2_BODY);
    pub static ref POX_2_TESTNET_CODE: String =
        format!("{}\n{}", BOOT_CODE_POX_TESTNET_CONSTS, POX_2_BODY);
    pub static ref BOOT_CODE_COST_VOTING_TESTNET: String = make_testnet_cost_voting();
    pub static ref STACKS_BOOT_CODE_MAINNET: [(&'static str, &'static str); 9] = [
        ("pox", &BOOT_CODE_POX_MAINNET),
        ("lockup", BOOT_CODE_LOCKUP),
        ("costs", BOOT_CODE_COSTS),
        ("cost-voting", BOOT_CODE_COST_VOTING_MAINNET),
        ("bns", &BOOT_CODE_BNS),
        ("genesis", &BOOT_CODE_GENESIS),
        ("costs-2", &BOOT_CODE_COSTS_2),
        ("costs-v2", &BOOT_CODE_COSTS_2), // for backwards compatibility with old Clarinet.toml files
        ("pox-2", &POX_2_MAINNET_CODE),
    ];
    pub static ref STACKS_BOOT_CODE_TESTNET: [(&'static str, &'static str); 9] = [
        ("pox", &BOOT_CODE_POX_TESTNET),
        ("lockup", BOOT_CODE_LOCKUP),
        ("costs", BOOT_CODE_COSTS),
        ("cost-voting", &BOOT_CODE_COST_VOTING_TESTNET),
        ("bns", &BOOT_CODE_BNS),
        ("genesis", &BOOT_CODE_GENESIS),
        ("costs-2", &BOOT_CODE_COSTS_2_TESTNET),
        ("costs-v2", &BOOT_CODE_COSTS_2_TESTNET), // for backwards compatibility with old Clarinet.toml files
        ("pox-2", &POX_2_TESTNET_CODE),
    ];
}

fn make_testnet_cost_voting() -> String {
    BOOT_CODE_COST_VOTING_MAINNET
        .replacen(
            "(define-constant VETO_LENGTH u1008)",
            "(define-constant VETO_LENGTH u50)",
            1,
        )
        .replacen(
            "(define-constant REQUIRED_VETOES u500)",
            "(define-constant REQUIRED_VETOES u25)",
            1,
        )
}
