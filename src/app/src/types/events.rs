use candid::CandidType;
use serde::{Deserialize, Serialize};

use super::address::Address;

#[derive(Clone, Debug, Serialize, Deserialize, CandidType)]
pub enum LedgerEvent {
    Coinbase { to: Address, amount: u128 },
    Transfer {
        from: Address,
        to: Address,
        amount: u128,
    },
}
