use crate::traits::CellStore;

#[derive(Debug)]
pub struct StructTxn<T: Clone, B: CellStore<T>> {
    base: B,
    overlays: Vec<Option<T>>, // top is last
}

impl<T: Clone, B: CellStore<T>> StructTxn<T, B> {
    pub fn new(base: B) -> Self {
        Self {
            base,
            overlays: vec![None],
        }
    }

    pub fn push_layer(&mut self) {
        self.overlays.push(None);
    }

    pub fn revert_top(&mut self) {
        if self.overlays.len() > 1 {
            self.overlays.pop();
        } else {
            self.overlays[0] = None;
        }
    }

    pub fn commit_top(&mut self) {
        if self.overlays.len() > 1 {
            let top = self.overlays.pop().unwrap();
            let next = self.overlays.last_mut().unwrap();
            if top.is_some() {
                *next = top;
            }
        } else {
            let top = self.overlays.pop().unwrap();
            if let Some(val) = top {
                self.base.set(val);
            }
            self.overlays.push(None);
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
        if let Some(val) = oldest {
            self.base.set(val);
        }
        if self.overlays.is_empty() {
            self.overlays.push(None);
        }
    }

    pub fn set(&mut self, v: T) {
        if let Some(top) = self.overlays.last_mut() {
            *top = Some(v);
        }
    }

    pub fn get(&self) -> Option<T> {
        for layer in self.overlays.iter().rev() {
            if let Some(v) = layer {
                return Some(v.clone());
            }
        }
        self.base.get()
    }
}
