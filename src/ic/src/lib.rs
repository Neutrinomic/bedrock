use app::reducer::reduce_in_order;
use app::store::StoreGeneric;
use app::types::{
    actions::{Action, ApplyStatus, LedgerAction, MetaAction},
    address::Address,
    block::Block,
    events::Event,
    meta::Meta,
};
use std::cell::RefCell;
use candid::CandidType;
use serde::{Deserialize, Serialize};

pub mod stable_backend;

type Store = StoreGeneric<
    stable_backend::StableMapBackend,
    stable_backend::StableCellBackend,
    stable_backend::StableLogBackend<Event>,
    stable_backend::StableLogBackend<Vec<u8>>,
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
fn get_event(i: usize) -> Option<Event> {
    STORE.with(|s| s.borrow().events.get(i))
}



#[ic_cdk::query]
fn meta_get_chain_name() -> Option<String> {
    STORE.with(|s| s.borrow().meta.get().map(|m| m.chain_name.clone()))
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
            let blk = Block { actions: actions.clone(), results: res.clone() };
            let bytes = candid::encode_one(&blk).expect("encode block");
            s.blocks.append(bytes);
            s.commit_top();
        }
        res
    })
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    with_store_mut(|s| s.commit_all());
}

#[ic_cdk::update]
fn clear_all() {
    with_store_mut(|s| {
        s.accounts.clear_all();
        s.meta.clear_all();
        s.events.clear_all();
        s.blocks.clear_all();
    });
}

#[ic_cdk::update]
fn reset_and_replay() {
    with_store_mut(|s| {
        // Clear current state (accounts/meta/events), keep blocks
        s.clear_state_preserve_blocks();
        // Re-apply all actions from blocks in a single layer
        s.push_layer();
        let total = s.blocks.len();
        for i in 0..total {
            if let Some(bytes) = s.blocks.get(i) {
                let block: Block = candid::decode_one(bytes.as_slice()).expect("decode block");
                for action in block.actions.iter() {
                    let _ = reduce_in_order(s, action);
                }
            }
        }
        s.commit_top();
    });
}

ic_cdk::export_candid!();

#[derive(CandidType, Serialize, Deserialize)]
struct BlocksPage {
    total: u64,
    start: u64,
    blocks: Vec<Block>,
}

#[ic_cdk::query]
fn get_blocks(start: u64, len: u64) -> BlocksPage {
    const MAX_BYTES: usize = 1_000_000; // ~1MB cap for blocks payload
    STORE.with(|s| {
        let store = s.borrow();
        let total = store.blocks.len() as u64;
        let start = start.min(total);
        let end = start.saturating_add(len).min(total);
        let mut blocks: Vec<Block> = Vec::new();
        let mut acc_bytes: usize = 0;
        for i in start..end {
            if let Some(bytes) = store.blocks.get(i as usize) {
                let sz = bytes.len();
                if acc_bytes + sz > MAX_BYTES && !blocks.is_empty() {
                    break;
                }
                let block: Block = candid::decode_one(bytes.as_slice()).unwrap_or_else(|_| Block { actions: vec![], results: vec![] });
                blocks.push(block);
                acc_bytes += sz;
            } else {
                break;
            }
        }
        BlocksPage { total, start, blocks }
    })
}
