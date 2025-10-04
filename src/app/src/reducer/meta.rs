use crate::store::StoreGeneric;
use crate::types::{
    actions::{Action, ApplyStatus, MetaAction},
    address::Address,
    events::{Event, MetaEvent},
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
    C: LogStore<Event>,
    D: LogStore<Vec<u8>>,
{
    match action {
        Action::Meta(MetaAction::SetChainName { name }) => {
            let mut m = store.meta.get().unwrap_or_else(Meta::default);
            m.chain_name = name.clone();
            store.meta.set(m.clone());
            store.events.append(Event::Meta(MetaEvent::SetChainName { name: m.chain_name }));
            ApplyStatus::Ok
        }
        Action::Meta(MetaAction::BumpCounter) => {
            let mut m = store.meta.get().unwrap_or_else(Meta::default);
            m.counter = m.counter.saturating_add(1);
            let new_counter = m.counter;
            store.meta.set(m);
            store
                .events
                .append(Event::Meta(MetaEvent::BumpCounter { new_counter }));
            ApplyStatus::Ok
        }
        _ => ApplyStatus::Pass { reason: "skipped by meta".into() },
    }
}
