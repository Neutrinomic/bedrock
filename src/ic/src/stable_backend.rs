use app::types::{address::Address, block::Block, events::LedgerEvent, meta::Meta};
use staging_memory::traits::{CellStore, LogStore, MapStore};

use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{cell::Cell as StableCell, log::Log as StableLog, BTreeMap as StableBTreeMap, DefaultMemoryImpl, Storable};

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: std::cell::RefCell<MemoryManager<DefaultMemoryImpl>> =
        std::cell::RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}

pub struct StableMapBackend {
    inner: StableBTreeMap<Vec<u8>, u128, Memory>,
}

impl StableMapBackend {
    pub fn new(mem: Memory) -> Self {
        let inner = StableBTreeMap::init(mem);
        Self { inner }
    }

    pub fn from_id(id: u8) -> Self {
        let mem = MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(id)));
        Self::new(mem)
    }
}

impl MapStore<Address, u128> for StableMapBackend {
    fn get(&self, k: &Address) -> Option<u128> {
        self.inner.get(&k.0)
    }

    fn put(&mut self, k: Address, v: u128) {
        self.inner.insert(k.0, v);
    }

    fn remove(&mut self, k: &Address) {
        self.inner.remove(&k.0);
    }

    fn keys(&self) -> Vec<Address> {
        self.inner.keys().map(Address).collect()
    }
}

pub struct StableCellBackend {
    inner: StableCell<Meta, Memory>,
}

impl StableCellBackend {
    pub fn new(mem: Memory) -> Self {
        let cell = StableCell::init(mem, Meta::default()).expect("init stable cell");
        Self { inner: cell }
    }

    pub fn from_id(id: u8) -> Self {
        let mem = MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(id)));
        Self::new(mem)
    }
}

impl CellStore<Meta> for StableCellBackend {
    fn get(&self) -> Option<Meta> {
        Some(self.inner.get().clone())
    }

    fn set(&mut self, v: Meta) {
        let _ = self.inner.set(v);
    }
}

pub struct StableLogBackend<T: Storable> {
    inner: StableLog<T, Memory, Memory>,
}

impl<T: Storable> StableLogBackend<T> {
    pub fn new(index_mem: Memory, data_mem: Memory) -> Self {
        let inner = StableLog::init(index_mem, data_mem).expect("init stable log");
        Self { inner }
    }

    pub fn from_ids(index_id: u8, data_id: u8) -> Self {
        let (index_mem, data_mem) = MEMORY_MANAGER.with(|m| {
            let mm = m.borrow();
            (mm.get(MemoryId::new(index_id)), mm.get(MemoryId::new(data_id)))
        });
        Self::new(index_mem, data_mem)
    }
}

impl<T> LogStore<T> for StableLogBackend<T>
where
    T: Storable + Clone,
{
    fn len(&self) -> usize {
        self.inner.len() as usize
    }

    fn get(&self, idx: usize) -> Option<T> {
        self.inner.get(idx as u64)
    }

    fn append(&mut self, v: T) {
        let _ = self.inner.append(&v);
    }

    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        for item in it {
            let _ = self.inner.append(&item);
        }
    }
}

pub fn make_stable_backends() -> (
    StableMapBackend,
    StableCellBackend,
    StableLogBackend<LedgerEvent>,
    StableLogBackend<Block>,
) {
    (
        StableMapBackend::from_id(0),
        StableCellBackend::from_id(1),
        StableLogBackend::from_ids(2, 3),
        StableLogBackend::from_ids(4, 5),
    )
}

