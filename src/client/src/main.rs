use candid::{decode_one, encode_one, encode_args};
use app::reducer::reduce_in_order;
use app::store::StoreGeneric;
use app::types::{
    actions::{Action, ApplyStatus},
    address::Address,
    block::Block,
    events::Event,
    meta::Meta,
};
use staging_memory::traits::{CellStore, LogStore, MapStore};
use sled::IVec;
use ic_agent::{Agent, agent::http_transport::ReqwestTransport};
use candid::Principal;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use std::io::Write;

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

    fn clear(&mut self) {
        let _ = self.tree.remove(b"value");
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

    fn clear(&mut self) {
        // Clear sled tree: naive approach: iterate keys and remove
        let keys: Vec<IVec> = self.tree.iter().keys().filter_map(|k| k.ok()).collect();
        for k in keys {
            let _ = self.tree.remove(k);
        }
        let _ = self.tree.insert(b"__len", 0u64.to_be_bytes().to_vec());
        let _ = self.tree.flush();
    }
}

// Raw-bytes log for blocks
struct DiskBytesLog {
    tree: sled::Tree,
}

impl DiskBytesLog {
    fn new(tree: sled::Tree) -> Self {
        if tree.get(b"__len").ok().flatten().is_none() {
            let _ = tree.insert(b"__len", 0u64.to_be_bytes().to_vec());
        }
        Self { tree }
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

impl LogStore<Vec<u8>> for DiskBytesLog {
    fn len(&self) -> usize {
        self.read_len() as usize
    }

    fn get(&self, idx: usize) -> Option<Vec<u8>> {
        let k = Self::idx_key(idx as u64);
        self.tree.get(k).ok().flatten().map(|ivec| ivec.to_vec())
    }

    fn append(&mut self, v: Vec<u8>) {
        let idx = self.read_len();
        let _ = self.tree.insert(Self::idx_key(idx), v);
        self.write_len(idx + 1);
    }

    fn extend<I: IntoIterator<Item = Vec<u8>>>(&mut self, it: I) {
        let mut idx = self.read_len();
        for v in it {
            let _ = self.tree.insert(Self::idx_key(idx), v);
            idx += 1;
        }
        self.write_len(idx);
    }

    fn clear(&mut self) {
        let keys: Vec<IVec> = self.tree.iter().keys().filter_map(|k| k.ok()).collect();
        for k in keys {
            let _ = self.tree.remove(k);
        }
        let _ = self.tree.insert(b"__len", 0u64.to_be_bytes().to_vec());
        let _ = self.tree.flush();
    }
}

type ClientStore = StoreGeneric<DiskMap, DiskCell<Meta>, DiskLog<Event>, DiskBytesLog>;

fn default_client_store(db: &sled::Db) -> ClientStore {
    let accounts = DiskMap::new(db.open_tree("accounts").expect("open accounts"));
    let meta = DiskCell::new(db.open_tree("meta").expect("open meta"));
    let events = DiskLog::new(db.open_tree("events").expect("open events"));
    let blocks = DiskBytesLog::new(db.open_tree("blocks").expect("open blocks"));

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
        let blk = Block { actions: actions.clone(), results: res.clone() };
        let bytes = encode_one(&blk).expect("encode block");
        store.blocks.append(bytes);
        store.commit_top();
    }
    res
}

#[derive(candid::CandidType, serde::Deserialize, serde::Serialize)]
struct BlocksPage {
    total: u64,
    start: u64,
    blocks: Vec<Block>,
}

async fn fetch_blocks(agent: &Agent, canister: Principal, start: u64, len: u64) -> Result<BlocksPage> {
    let arg = encode_args((start, len))?;
    let bytes = agent.query(&canister, "get_blocks").with_arg(arg).call().await?;
    let page: BlocksPage = decode_one(bytes.as_slice())?;
    Ok(page)
}

fn get_canister_id() -> Result<Principal> {
    if let Ok(id) = std::env::var("CANISTER_ID_APPCHAIN") {
        Ok(Principal::from_text(id)?)
    } else if let Ok(id) = std::env::var("CANISTER_ID") {
        Ok(Principal::from_text(id)?)
    } else {
        anyhow::bail!("CANISTER_ID_APPCHAIN not set; run `dfx deploy` to generate .env")
    }
}

async fn build_agent_for_url(url: &str) -> Result<Agent> {
    let transport = ReqwestTransport::create(url.to_string())?;
    let agent = Agent::builder().with_transport(transport).build()?;
    // local dev only
    let _ = agent.fetch_root_key().await;
    // probe status
    let _ = agent.status().await?;
    Ok(agent)
}

async fn build_agent() -> Result<(Agent, String)> {
    if let Ok(url) = std::env::var("DFX_REPLICA_ADDRESS") {
        if let Ok(agent) = build_agent_for_url(&url).await {
            return Ok((agent, url));
        }
    }
    if let Ok(url) = std::env::var("DFX_UI_ADDRESS") {
        if let Ok(agent) = build_agent_for_url(&url).await {
            return Ok((agent, url));
        }
    }
    for url in [
        "http://127.0.0.1:4943",
        "http://127.0.0.1:8000",
        "http://127.0.0.1:8080",
    ]
    .iter()
    {
        if let Ok(agent) = build_agent_for_url(url).await {
            return Ok((agent, (*url).to_string()));
        }
    }
    anyhow::bail!(
        "Could not connect to local replica at 4943/8000/8080; set DFX_REPLICA_ADDRESS or DFX_UI_ADDRESS"
    )
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // Load .env generated by dfx deploy
    let _ = dotenvy::dotenv();
    let db = sled::open("client_db").expect("open sled");
    let mut store = default_client_store(&db);
    println!("client initialized; existing blocks(local): {}", store.blocks.len());

    let canister = get_canister_id()?;
    let (agent, url) = build_agent().await?;
    println!("Connected to replica at {url}");

    // Determine next from local persisted blocks; also rebuild local state from local blocks
    let local_blocks = store.blocks.len() as u64;
    if local_blocks > 0 {
        println!("Replaying {} local blocks to rebuild state...", local_blocks);
        store.clear_state_preserve_blocks();
        store.push_layer();
        for i in 0..local_blocks {
            if let Some(bytes) = store.blocks.get(i as usize) {
                let blk: Block = decode_one(bytes.as_slice()).expect("decode local block");
                for action in blk.actions.iter() {
                    let _ = reduce_in_order(&mut store, action);
                }
            }
        }
        store.commit_top();
        let counter = store.meta.get().map(|m| m.counter).unwrap_or(0);
        println!("Local state rebuilt. events_local={} counter_local={}", store.events.len(), counter);
    }
    let mut next: u64 = local_blocks;

    loop {
        match fetch_blocks(&agent, canister, next, 1000).await {
            Ok(page) => {
                let count = page.blocks.len() as u64;
                if count == 0 {
                    print!("."); let _ = std::io::stdout().flush();
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }

                store.push_layer();
                for blk in page.blocks.iter() {
                    for action in blk.actions.iter() {
                        let _ = reduce_in_order(&mut store, action);
                    }
                    let bytes = encode_one(blk).expect("encode block");
                    store.blocks.append(bytes);
                }
                store.commit_top();

                next += count;
                let counter = store.meta.get().map(|m| m.counter).unwrap_or(0);
                println!(
                    "\nApplied {} blocks; next={} total_remote={} events_local={} counter_local={}",
                    count,
                    next,
                    page.total,
                    store.events.len(),
                    counter
                );
            }
            Err(err) => {
                eprintln!("fetch error at start={next}: {err:?}");
                sleep(Duration::from_secs(3)).await;
            }
        }
    }
}
