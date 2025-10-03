use candid::CandidType;
use serde::{Deserialize, Serialize};

use super::address::Address;

#[derive(Clone, Debug, Default, Serialize, Deserialize, CandidType)]
pub struct Meta {
    pub chain_name: String,
    pub owner: Option<Address>,
    pub counter: u64,
}
