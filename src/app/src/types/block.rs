use candid::CandidType;
use serde::{Deserialize, Serialize};

use super::actions::{Action, ApplyStatus};

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct Block {
    pub actions: Vec<Action>,
    pub results: Vec<ApplyStatus>,
}
