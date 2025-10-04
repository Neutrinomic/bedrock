use std::collections::BTreeMap;

use crate::traits::{CellStore, LogStore, MapStore};

#[derive(Debug, Default)]
pub struct InMemoryMap<K, V>
where
    K: Ord + Clone,
    V: Clone,
{
    inner: BTreeMap<K, V>,
}

impl<K, V> InMemoryMap<K, V>
where
    K: Ord + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }
}

impl<K, V> MapStore<K, V> for InMemoryMap<K, V>
where
    K: Ord + Clone,
    V: Clone,
{
    fn get(&self, k: &K) -> Option<V> {
        self.inner.get(k).cloned()
    }

    fn put(&mut self, k: K, v: V) {
        self.inner.insert(k, v);
    }

    fn remove(&mut self, k: &K) {
        self.inner.remove(k);
    }

    fn keys(&self) -> Vec<K> {
        self.inner.keys().cloned().collect()
    }

    fn clear(&mut self) {
        self.inner.clear();
    }
}

#[derive(Debug, Default)]
pub struct InMemoryCell<T>
where
    T: Clone,
{
    inner: Option<T>,
}

impl<T> InMemoryCell<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self { inner: None }
    }
}

impl<T> CellStore<T> for InMemoryCell<T>
where
    T: Clone,
{
    fn get(&self) -> Option<T> {
        self.inner.clone()
    }

    fn set(&mut self, v: T) {
        self.inner = Some(v);
    }

    fn clear(&mut self) {
        self.inner = None;
    }
}

#[derive(Debug, Default)]
pub struct InMemoryLog<T>
where
    T: Clone,
{
    inner: Vec<T>,
}

impl<T> InMemoryLog<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

impl<T> LogStore<T> for InMemoryLog<T>
where
    T: Clone,
{
    fn len(&self) -> usize {
        self.inner.len()
    }

    fn get(&self, idx: usize) -> Option<T> {
        self.inner.get(idx).cloned()
    }

    fn append(&mut self, v: T) {
        self.inner.push(v);
    }

    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        self.inner.extend(it);
    }

    fn clear(&mut self) {
        self.inner.clear();
    }
}
