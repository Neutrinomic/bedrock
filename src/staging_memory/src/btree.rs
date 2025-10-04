use crate::{overlay::Overlay, traits::MapStore};

#[derive(Debug)]
pub struct BTreeTxn<K, V, B>
where
    K: Ord + Clone,
    V: Clone,
    B: MapStore<K, V>,
{
    base: B,
    overlays: Vec<Overlay<K, V>>, // top is last
}

impl<K, V, B> BTreeTxn<K, V, B>
where
    K: Ord + Clone,
    V: Clone,
    B: MapStore<K, V>,
{
    pub fn new(base: B) -> Self {
        Self {
            base,
            overlays: vec![Overlay::new()],
        }
    }

    pub fn push_layer(&mut self) {
        self.overlays.push(Overlay::new());
    }

    pub fn revert_top(&mut self) {
        if self.overlays.len() > 1 {
            self.overlays.pop();
        } else {
            self.overlays[0].staged.clear();
        }
    }

    pub fn commit_top(&mut self) {
        if self.overlays.len() > 1 {
            let top = self.overlays.pop().unwrap();
            let next = self.overlays.last_mut().unwrap();
            for (k, v) in top.staged {
                next.staged.insert(k, v);
            }
        } else {
            let top = self.overlays.pop().unwrap();
            for (k, v) in top.staged {
                match v {
                    Some(val) => self.base.put(k, val),
                    None => self.base.remove(&k),
                }
            }
            self.overlays.push(Overlay::new());
        }
    }

    pub fn commit_all(&mut self) {
        while self.overlays.len() > 1 {
            self.commit_top();
        }
        self.commit_top();
    }

    pub fn commit_oldest(&mut self) {
        if self.overlays.is_empty() {
            return;
        }
        let oldest = self.overlays.remove(0);
        for (k, v) in oldest.staged {
            match v {
                Some(val) => self.base.put(k, val),
                None => self.base.remove(&k),
            }
        }
        if self.overlays.is_empty() {
            self.overlays.push(Overlay::new());
        }
    }

    pub fn insert(&mut self, k: K, v: V) {
        self.overlays
            .last_mut()
            .expect("at least one layer")
            .staged
            .insert(k, Some(v));
    }

    pub fn remove(&mut self, k: &K) {
        self.overlays
            .last_mut()
            .expect("at least one layer")
            .staged
            .insert(k.clone(), None);
    }

    pub fn get(&self, k: &K) -> Option<V> {
        for layer in self.overlays.iter().rev() {
            if let Some(v) = layer.staged.get(k) {
                return v.clone();
            }
        }
        self.base.get(k)
    }

    pub fn iter_effective<'a>(&'a self) -> BTreeEffectiveIter<'a, K, V, B> {
        BTreeEffectiveIter::new(self)
    }

    pub fn base_len(&self) -> usize {
        self.base.keys().len()
    }

    pub fn clear_all(&mut self) {
        self.base.clear();
        self.overlays.clear();
        self.overlays.push(Overlay::new());
    }
}

pub struct BTreeEffectiveIter<'a, K, V, B>
where
    K: Ord + Clone,
    V: Clone,
    B: MapStore<K, V>,
{
    all_keys: Vec<K>,
    idx: usize,
    txn: &'a BTreeTxn<K, V, B>,
}

impl<'a, K, V, B> BTreeEffectiveIter<'a, K, V, B>
where
    K: Ord + Clone,
    V: Clone,
    B: MapStore<K, V>,
{
    fn new(txn: &'a BTreeTxn<K, V, B>) -> Self {
        use std::collections::BTreeMap;
        let mut keys: BTreeMap<K, ()> = BTreeMap::new();
        for k in txn.base.keys().into_iter() {
            keys.insert(k, ());
        }
        for layer in &txn.overlays {
            for k in layer.staged.keys().cloned() {
                keys.insert(k, ());
            }
        }
        Self {
            all_keys: keys.into_keys().collect(),
            idx: 0,
            txn,
        }
    }
}

impl<'a, K, V, B> Iterator for BTreeEffectiveIter<'a, K, V, B>
where
    K: Ord + Clone,
    V: Clone,
    B: MapStore<K, V>,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        while self.idx < self.all_keys.len() {
            let k = self.all_keys[self.idx].clone();
            self.idx += 1;
            if let Some(v) = self.txn.get(&k) {
                return Some((k, v));
            }
        }
        None
    }
}
