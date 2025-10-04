use std::borrow::Cow;

use candid::{decode_one, encode_one};
use ic_stable_structures::{storable::Bound, Storable};

use super::{block::Block, events::{Event, LedgerEvent, MetaEvent}, meta::Meta};

impl Storable for Meta {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(encode_one(self).expect("candid encode Meta"))
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        decode_one(bytes.as_ref()).expect("candid decode Meta")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl Storable for Event {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(encode_one(self).expect("candid encode Event"))
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        decode_one(bytes.as_ref()).expect("candid decode Event")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl Storable for Block {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(encode_one(self).expect("candid encode Block"))
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        decode_one(bytes.as_ref()).expect("candid decode Block")
    }

    const BOUND: Bound = Bound::Unbounded;
}
