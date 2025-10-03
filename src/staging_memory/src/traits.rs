pub trait MapStore<K, V>
where
    K: Ord + Clone,
    V: Clone,
{
    fn get(&self, k: &K) -> Option<V>;
    fn put(&mut self, k: K, v: V);
    fn remove(&mut self, k: &K);
    fn keys(&self) -> Vec<K>;
}

pub trait CellStore<T>
where
    T: Clone,
{
    fn get(&self) -> Option<T>;
    fn set(&mut self, v: T);
}

pub trait LogStore<T>
where
    T: Clone,
{
    fn len(&self) -> usize;
    fn get(&self, idx: usize) -> Option<T>;
    fn append(&mut self, v: T);
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I);
}

