use candid::{decode_one, encode_one};
use app::reducer::reduce_in_order;
use app::store::StoreGeneric;
use app::types::{
    actions::{Action, ApplyStatus},
    address::Address,
    block::Block,
    events::LedgerEvent,
    meta::Meta,
};
use staging_memory::traits::{CellStore, LogStore, MapStore};

// Disk-backed Map for Address -> u128 using sled
struct DiskMap {
    tree: sled::Tree,
}

impl DiskMap {
    fn new(tree: sled::Tree) -> Self {
        Self { tree }
    }
}

impl MapStore<Address, u128> for DiskMap {
    fn get(&self, k: &Address) -> Option<u128> {
        self.tree.get(&k.0).ok().flatten().map(|ivec| {
            let bytes: [u8; 16] = ivec.as_ref().try_into().unwrap();
            u128::from_be_bytes(bytes)
        })
    }

    fn put(&mut self, k: Address, v: u128) {
        let _ = self.tree.insert(k.0, v.to_be_bytes().to_vec());
        let _ = self.tree.flush();
    }

    fn remove(&mut self, k: &Address) {
        let _ = self.tree.remove(&k.0);
        let _ = self.tree.flush();
    }

    fn keys(&self) -> Vec<Address> {
        self.tree
            .iter()
            .keys()
            .filter_map(|k| k.ok())
            .map(|ivec| Address(ivec.to_vec()))
            .collect()
    }
}

// Disk-backed Cell for Meta using sled
struct DiskCell<T> {
    tree: sled::Tree,
    _marker: std::marker::PhantomData<T>,
}

impl<T> DiskCell<T> {
    fn new(tree: sled::Tree) -> Self {
        Self { tree, _marker: std::marker::PhantomData }
    }
}

impl<T> CellStore<T> for DiskCell<T>
where
    T: candid::CandidType + serde::de::DeserializeOwned + Clone + serde::Serialize,
{
    fn get(&self) -> Option<T> {
        self.tree.get(b"value").ok().flatten().map(|ivec| {
            decode_one::<T>(ivec.as_ref()).expect("decode cell")
        })
    }

    fn set(&mut self, v: T) {
        let bytes = encode_one(&v).expect("encode cell");
        let _ = self.tree.insert(b"value", bytes);
        let _ = self.tree.flush();
    }
}

// Disk-backed Log using sled
struct DiskLog<T> {
    tree: sled::Tree,
    _marker: std::marker::PhantomData<T>,
}

impl<T> DiskLog<T> {
    fn new(tree: sled::Tree) -> Self {
        // ensure len key exists
        if tree.get(b"__len").ok().flatten().is_none() {
            let _ = tree.insert(b"__len", 0u64.to_be_bytes().to_vec());
        }
        Self { tree, _marker: std::marker::PhantomData }
    }

    fn read_len(&self) -> u64 {
        self.tree
            .get(b"__len")
            .ok()
            .flatten()
            .map(|ivec| u64::from_be_bytes(ivec.as_ref().try_into().unwrap()))
            .unwrap_or(0)
    }

    fn write_len(&self, len: u64) {
        let _ = self.tree.insert(b"__len", len.to_be_bytes().to_vec());
        let _ = self.tree.flush();
    }

    fn idx_key(idx: u64) -> [u8; 8] { idx.to_be_bytes() }
}

impl<T> LogStore<T> for DiskLog<T>
where
    T: candid::CandidType + serde::de::DeserializeOwned + Clone + serde::Serialize,
{
    fn len(&self) -> usize {
        self.read_len() as usize
    }

    fn get(&self, idx: usize) -> Option<T> {
        let k = Self::idx_key(idx as u64);
        self.tree.get(k).ok().flatten().map(|ivec| decode_one(ivec.as_ref()).expect("decode log item"))
    }

    fn append(&mut self, v: T) {
        let idx = self.read_len();
        let _ = self.tree.insert(Self::idx_key(idx), encode_one(&v).expect("encode log item"));
        self.write_len(idx + 1);
    }

    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        let mut idx = self.read_len();
        for v in it {
            let _ = self.tree.insert(Self::idx_key(idx), encode_one(&v).expect("encode log item"));
            idx += 1;
        }
        self.write_len(idx);
    }
}

type ClientStore = StoreGeneric<DiskMap, DiskCell<Meta>, DiskLog<LedgerEvent>, DiskLog<Block>>;

fn default_client_store(db: &sled::Db) -> ClientStore {
    let accounts = DiskMap::new(db.open_tree("accounts").expect("open accounts"));
    let meta = DiskCell::new(db.open_tree("meta").expect("open meta"));
    let events = DiskLog::new(db.open_tree("events").expect("open events"));
    let blocks = DiskLog::new(db.open_tree("blocks").expect("open blocks"));

    StoreGeneric::new(accounts, meta, events, blocks)
}

fn apply_block_local(store: &mut ClientStore, actions: Vec<Action>) -> Vec<ApplyStatus> {
    store.push_layer();
    let mut res = Vec::with_capacity(actions.len());
    let mut any_err = false;
    for a in actions.iter() {
        let mut saw_ok = false;
        let mut err: Option<String> = None;
        for reducer in [reduce_in_order] {
            match reducer(store, a) {
                ApplyStatus::Ok => saw_ok = true,
                ApplyStatus::Pass { .. } => {}
                ApplyStatus::Err { error } => err = Some(error),
            }
        }
        let status = if let Some(e) = err {
            any_err = true;
            ApplyStatus::Err { error: e }
        } else if saw_ok {
            ApplyStatus::Ok
        } else {
            ApplyStatus::Pass { reason: "no reducer handled action".into() }
        };
        res.push(status);
    }
    if any_err {
        store.revert_top();
    } else {
        store.blocks.append(Block { actions: actions.clone(), results: res.clone() });
        store.commit_top();
    }
    res
}

fn main() {
    let db = sled::open("client_db").expect("open sled");
    let mut store = default_client_store(&db);
    println!("client initialized; existing blocks: {}", store.blocks.len());
}
