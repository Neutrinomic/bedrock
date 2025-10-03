use crate::store::StoreGeneric;
use crate::types::{
    actions::{Action, ApplyStatus, LedgerAction},
    address::Address,
    block::Block,
    events::LedgerEvent,
    meta::Meta,
};
use staging_memory::traits::{CellStore, LogStore, MapStore};

pub fn reduce<A, B, C, D>(
    store: &mut StoreGeneric<A, B, C, D>,
    action: &Action,
) -> ApplyStatus
where
    A: MapStore<Address, u128>,
    B: CellStore<Meta>,
    C: LogStore<LedgerEvent>,
    D: LogStore<Block>,
{
    match action {
        Action::Ledger(LedgerAction::Coinbase { to, amount }) => {
            let cur = store.accounts.get(to).unwrap_or(0);
            store.accounts.insert(to.clone(), cur.saturating_add(*amount));
            store
                .events
                .append(LedgerEvent::Coinbase { to: to.clone(), amount: *amount });
            ApplyStatus::Ok
        }
        Action::Ledger(LedgerAction::Transfer { from, to, amount }) => {
            if *amount == 0 {
                return ApplyStatus::Pass {
                    reason: "zero-amount transfer".into(),
                };
            }
            let from_bal = store.accounts.get(from).unwrap_or(0);
            if from_bal < *amount {
                return ApplyStatus::Pass {
                    reason: "insufficient funds".into(),
                };
            }
            let to_bal = store.accounts.get(to).unwrap_or(0);
            store
                .accounts
                .insert(from.clone(), from_bal.saturating_sub(*amount));
            store
                .accounts
                .insert(to.clone(), to_bal.saturating_add(*amount));
            store
                .events
                .append(LedgerEvent::Transfer { from: from.clone(), to: to.clone(), amount: *amount });
            ApplyStatus::Ok
        }
        _ => ApplyStatus::Pass { reason: "skipped by ledger".into() },
    }
}
