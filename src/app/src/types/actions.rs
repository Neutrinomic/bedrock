use candid::CandidType;
use serde::{Deserialize, Serialize};

use super::address::Address;

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub enum LedgerAction {
    Coinbase { to: Address, amount: u128 },
    Transfer { from: Address, to: Address, amount: u128 },
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub enum MetaAction {
    SetChainName { name: String },
    BumpCounter,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub enum Action {
    Ledger(LedgerAction),
    Meta(MetaAction),
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub enum ApplyStatus {
    Ok,
    Pass { reason: String },
    Err { error: String },
}
