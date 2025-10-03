use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize, CandidType)]
pub struct Address(pub Vec<u8>);

impl From<Vec<u8>> for Address {
    fn from(v: Vec<u8>) -> Self {
        Address(v)
    }
}

impl Address {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
