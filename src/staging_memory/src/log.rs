use crate::traits::LogStore;

#[derive(Debug)]
pub struct LogTxn<T: Clone, B: LogStore<T>> {
    base: B,
    overlays: Vec<Vec<T>>, // top is last
}

impl<T: Clone, B: LogStore<T>> LogTxn<T, B> {
    pub fn new(base: B) -> Self {
        Self {
            base,
            overlays: vec![Vec::new()],
        }
    }

    pub fn push_layer(&mut self) {
        self.overlays.push(Vec::new());
    }

    pub fn revert_top(&mut self) {
        if self.overlays.len() > 1 {
            self.overlays.pop();
        } else {
            self.overlays[0].clear();
        }
    }

    pub fn commit_top(&mut self) {
        if self.overlays.len() > 1 {
            let top = self.overlays.pop().unwrap();
            let next = self.overlays.last_mut().unwrap();
            next.extend(top);
        } else {
            let top = self.overlays.pop().unwrap();
            self.base.extend(top);
            self.overlays.push(Vec::new());
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
        self.base.extend(oldest);
        if self.overlays.is_empty() {
            self.overlays.push(Vec::new());
        }
    }

    pub fn append(&mut self, v: T) {
        if let Some(top) = self.overlays.last_mut() {
            top.push(v);
        }
    }

    pub fn len(&self) -> usize {
        self.base.len() + self.overlays.iter().map(|v| v.len()).sum::<usize>()
    }

    pub fn get(&self, idx: usize) -> Option<T> {
        if idx < self.base.len() {
            return self.base.get(idx);
        }
        let mut remaining = idx - self.base.len();
        for layer in &self.overlays {
            if remaining < layer.len() {
                return layer.get(remaining).cloned();
            }
            remaining -= layer.len();
        }
        None
    }

    pub fn clear(&mut self) {
        self.base.clear();
        for layer in &mut self.overlays {
            layer.clear();
        }
    }

    pub fn clear_all(&mut self) {
        self.clear();
        self.overlays.clear();
        self.overlays.push(Vec::new());
    }
}
