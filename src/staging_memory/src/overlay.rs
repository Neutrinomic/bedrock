use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Overlay<K, V> {
    pub staged: BTreeMap<K, Option<V>>,
}

impl<K: Ord, V> Overlay<K, V> {
    pub fn new() -> Self {
        Self {
            staged: BTreeMap::new(),
        }
    }
}

