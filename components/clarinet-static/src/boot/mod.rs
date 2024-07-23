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

const BOOT_CODE_GENESIS: &str = std::include_str!("genesis.clar");
const BOOT_CODE_BNS: &str = std::include_str!("bns.clar");
const BOOT_CODE_LOCKUP: &str = std::include_str!("lockup.clar");

pub const BOOT_CODE_COSTS: &str = std::include_str!("costs.clar");
pub const BOOT_CODE_COSTS_2: &str = std::include_str!("costs-2.clar");
pub const BOOT_CODE_COSTS_2_TESTNET: &str = std::include_str!("costs-2-testnet.clar");
pub const BOOT_CODE_COSTS_3: &str = std::include_str!("costs-3.clar");
const BOOT_CODE_COST_VOTING_MAINNET: &str = std::include_str!("cost-voting.clar");

const BOOT_CODE_POX_TESTNET_CONSTS: &str = std::include_str!("pox-testnet.clar");
const BOOT_CODE_POX_MAINNET_CONSTS: &str = std::include_str!("pox-mainnet.clar");
const BOOT_CODE_POX_BODY: &str = std::include_str!("pox.clar");
const POX_2_BODY: &str = std::include_str!("pox-2.clar");
const POX_3_BODY: &str = std::include_str!("pox-3.clar");
const POX_4_BODY: &str = std::include_str!("pox-4.clar");

pub const BOOT_CODE_SIGNERS: &str = std::include_str!("signers.clar");
pub const BOOT_CODE_SIGNERS_VOTING: &str = std::include_str!("signers-voting.clar");

lazy_static! {
    pub static ref BOOT_CODE_POX_MAINNET: String =
        format!("{}\n{}", BOOT_CODE_POX_MAINNET_CONSTS, BOOT_CODE_POX_BODY);
    pub static ref BOOT_CODE_POX_TESTNET: String =
        format!("{}\n{}", BOOT_CODE_POX_TESTNET_CONSTS, BOOT_CODE_POX_BODY);
    pub static ref POX_2_MAINNET_CODE: String =
        format!("{}\n{}", BOOT_CODE_POX_MAINNET_CONSTS, POX_2_BODY);
    pub static ref POX_2_TESTNET_CODE: String =
        format!("{}\n{}", BOOT_CODE_POX_TESTNET_CONSTS, POX_2_BODY);
    pub static ref POX_3_MAINNET_CODE: String =
        format!("{}\n{}", BOOT_CODE_POX_MAINNET_CONSTS, POX_3_BODY);
    pub static ref POX_3_TESTNET_CODE: String =
        format!("{}\n{}", BOOT_CODE_POX_TESTNET_CONSTS, POX_3_BODY);
    pub static ref BOOT_CODE_COST_VOTING_TESTNET: String = make_testnet_cost_voting();
    pub static ref STACKS_BOOT_CODE_MAINNET: [(&'static str, &'static str); 13] = [
        ("pox", &BOOT_CODE_POX_MAINNET),
        ("lockup", BOOT_CODE_LOCKUP),
        ("costs", BOOT_CODE_COSTS),
        ("cost-voting", BOOT_CODE_COST_VOTING_MAINNET),
        ("bns", BOOT_CODE_BNS),
        ("genesis", BOOT_CODE_GENESIS),
        ("costs-2", BOOT_CODE_COSTS_2),
        ("pox-2", &POX_2_MAINNET_CODE),
        ("costs-3", BOOT_CODE_COSTS_3),
        ("pox-3", &POX_3_MAINNET_CODE),
        ("pox-4", POX_4_BODY),
        ("signers", BOOT_CODE_SIGNERS),
        ("signers-voting", BOOT_CODE_SIGNERS_VOTING),
    ];
    pub static ref STACKS_BOOT_CODE_TESTNET: [(&'static str, &'static str); 13] = [
        ("pox", &BOOT_CODE_POX_TESTNET),
        ("lockup", BOOT_CODE_LOCKUP),
        ("costs", BOOT_CODE_COSTS),
        ("cost-voting", &BOOT_CODE_COST_VOTING_TESTNET),
        ("bns", BOOT_CODE_BNS),
        ("genesis", BOOT_CODE_GENESIS),
        ("costs-2", BOOT_CODE_COSTS_2_TESTNET),
        ("pox-2", &POX_2_TESTNET_CODE),
        ("costs-3", BOOT_CODE_COSTS_3),
        ("pox-3", &POX_3_TESTNET_CODE),
        ("pox-4", POX_4_BODY),
        ("signers", BOOT_CODE_SIGNERS),
        ("signers-voting", BOOT_CODE_SIGNERS_VOTING),
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
