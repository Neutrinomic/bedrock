use staging_memory::{
    btree::BTreeTxn, log::LogTxn, struct_store::StructTxn, traits::{CellStore, LogStore, MapStore}
};
use crate::types::{address::Address, block::Block, events::Event, meta::Meta};

#[derive(Debug)]
pub struct StoreGeneric<A, B, C, D>
where
    A: MapStore<Address, u128>,
    B: CellStore<Meta>,
    C: LogStore<Event>,
    D: LogStore<Vec<u8>>,
{
    pub accounts: BTreeTxn<Address, u128, A>,
    pub meta: StructTxn<Meta, B>,
    pub events: LogTxn<Event, C>,
    pub blocks: LogTxn<Vec<u8>, D>,
}

impl<A, B, C, D> StoreGeneric<A, B, C, D>
where
    A: MapStore<Address, u128>,
    B: CellStore<Meta>,
    C: LogStore<Event>,
    D: LogStore<Vec<u8>>,
{
    pub fn new(accounts_base: A, meta_base: B, events_base: C, blocks_base: D) -> Self {
        Self {
            accounts: BTreeTxn::new(accounts_base),
            meta: StructTxn::new(meta_base),
            events: LogTxn::new(events_base),
            blocks: LogTxn::new(blocks_base),
        }
    }

    pub fn push_layer(&mut self) {
        self.accounts.push_layer();
        self.meta.push_layer();
        self.events.push_layer();
        self.blocks.push_layer();
    }

    pub fn revert_top(&mut self) {
        self.accounts.revert_top();
        self.meta.revert_top();
        self.events.revert_top();
        self.blocks.revert_top();
    }

    pub fn commit_top(&mut self) {
        self.accounts.commit_top();
        self.meta.commit_top();
        self.events.commit_top();
        self.blocks.commit_top();
    }

    pub fn commit_all(&mut self) {
        self.accounts.commit_all();
        self.meta.commit_all();
        self.events.commit_all();
        self.blocks.commit_all();
    }

    pub fn commit_oldest(&mut self) {
        self.accounts.commit_oldest();
        self.meta.commit_oldest();
        self.events.commit_oldest();
        self.blocks.commit_oldest();
    }

    pub fn clear_state_preserve_blocks(&mut self) {
        self.accounts.clear_all();
        self.meta.clear_all();
        self.events.clear_all();
    }
}

// Concrete Store type and default_store are defined in the IC crate and client crate.
