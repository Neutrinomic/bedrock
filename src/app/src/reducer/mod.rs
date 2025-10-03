pub mod ledger;
pub mod meta;

use crate::store::StoreGeneric;
use crate::types::{
    actions::{Action, ApplyStatus},
    address::Address,
    block::Block,
    events::LedgerEvent,
    meta::Meta,
};
use staging_memory::traits::{CellStore, LogStore, MapStore};

pub fn reduce_in_order<A, B, C, D>(
    store: &mut StoreGeneric<A, B, C, D>,
    action: &Action,
) -> ApplyStatus
where
    A: MapStore<Address, u128>,
    B: CellStore<Meta>,
    C: LogStore<LedgerEvent>,
    D: LogStore<Block>,
{
    let mut saw_ok = false;
    let mut err: Option<String> = None;
    for reducer in [ledger::reduce::<A, B, C, D>, meta::reduce::<A, B, C, D>] {
        match reducer(store, action) {
            ApplyStatus::Ok => saw_ok = true,
            ApplyStatus::Pass { .. } => {}
            ApplyStatus::Err { error } => err = Some(error),
        }
    }
    if let Some(e) = err {
        ApplyStatus::Err { error: e }
    } else if saw_ok {
        ApplyStatus::Ok
    } else {
        ApplyStatus::Pass {
            reason: "no reducer handled action".into(),
        }
    }
}
