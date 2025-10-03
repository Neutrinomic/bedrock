use app::reducer::reduce_in_order;
use app::store::StoreGeneric;
use app::types::{
    actions::{Action, ApplyStatus, LedgerAction, MetaAction},
    address::Address,
    block::Block,
    events::LedgerEvent,
    meta::Meta,
};
use std::cell::RefCell;

pub mod stable_backend;

type Store = StoreGeneric<
    stable_backend::StableMapBackend,
    stable_backend::StableCellBackend,
    stable_backend::StableLogBackend<LedgerEvent>,
    stable_backend::StableLogBackend<Block>,
>;

fn default_store() -> Store {
    let (accounts, meta, events, blocks) = stable_backend::make_stable_backends();
    StoreGeneric::new(accounts, meta, events, blocks)
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(default_store());
}

fn with_store_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut Store) -> R,
{
    STORE.with(|s| f(&mut *s.borrow_mut()))
}

#[ic_cdk::update]
fn txn_push_layer() {
    with_store_mut(|s| s.push_layer());
}

#[ic_cdk::update]
fn txn_commit_top() {
    with_store_mut(|s| s.commit_top());
}

#[ic_cdk::update]
fn txn_commit_all() {
    with_store_mut(|s| s.commit_all());
}

#[ic_cdk::update]
fn txn_commit_oldest() {
    with_store_mut(|s| s.commit_oldest());
}

#[ic_cdk::update]
fn txn_revert_top() {
    with_store_mut(|s| s.revert_top());
}

#[ic_cdk::update]
fn ledger_coinbase(to: Vec<u8>, amount: u128) {
    with_store_mut(|s| {
        let addr = Address::from(to);
        let _ = app::reducer::ledger::reduce(
            s,
            &Action::Ledger(LedgerAction::Coinbase { to: addr, amount }),
        );
    })
}

#[ic_cdk::update]
fn ledger_transfer(from: Vec<u8>, to: Vec<u8>, amount: u128) -> Result<(), String> {
    with_store_mut(|s| {
        let from = Address::from(from);
        let to = Address::from(to);
        match app::reducer::ledger::reduce(
            s,
            &Action::Ledger(LedgerAction::Transfer { from, to, amount }),
        ) {
            ApplyStatus::Ok => Ok(()),
            ApplyStatus::Pass { reason } => Err(reason),
            ApplyStatus::Err { error } => Err(error),
        }
    })
}

#[ic_cdk::query]
fn get_balance(addr: Vec<u8>) -> u128 {
    STORE.with(|s| {
        let a = Address::from(addr);
        s.borrow().accounts.get(&a).unwrap_or(0)
    })
}

#[ic_cdk::query]
fn events_len() -> usize {
    STORE.with(|s| s.borrow().events.len())
}

#[ic_cdk::query]
fn get_event(i: usize) -> Option<LedgerEvent> {
    STORE.with(|s| s.borrow().events.get(i))
}

#[ic_cdk::update]
fn meta_set_chain_name(name: String) {
    with_store_mut(|s| {
        let _ = app::reducer::meta::reduce(s, &Action::Meta(MetaAction::SetChainName { name }));
    })
}

#[ic_cdk::query]
fn meta_get_chain_name() -> Option<String> {
    STORE.with(|s| s.borrow().meta.get().map(|m| m.chain_name.clone()))
}

#[ic_cdk::update]
fn meta_bump_counter() -> u64 {
    with_store_mut(|s| {
        let _ = app::reducer::meta::reduce(s, &Action::Meta(MetaAction::BumpCounter));
        s.meta.get().map(|m| m.counter).unwrap_or(0)
    })
}

#[ic_cdk::query]
fn meta_get_counter() -> u64 {
    STORE.with(|s| s.borrow().meta.get().map(|m| m.counter).unwrap_or(0))
}

#[ic_cdk::update]
fn apply_block(actions: Vec<Action>) -> Vec<ApplyStatus> {
    with_store_mut(|s| {
        s.push_layer();
        let mut res = Vec::with_capacity(actions.len());
        let mut any_err = false;
        for a in actions.iter() {
            let status = reduce_in_order(s, a);
            if let ApplyStatus::Err { .. } = status {
                any_err = true;
            }
            res.push(status);
        }
        if any_err {
            s.revert_top();
        } else {
            s.blocks
                .append(Block { actions: actions.clone(), results: res.clone() });
            s.commit_top();
        }
        res
    })
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    with_store_mut(|s| s.commit_all());
}

ic_cdk::export_candid!();

