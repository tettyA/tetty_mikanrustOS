#![allow(non_snake_case, dead_code)]

use core::array;

pub struct ArrayMap<K, V, const N: usize = 16> {
    table_: [Option<(K, V)>; N],
}

impl<K: Copy + PartialEq, V: Copy, const N: usize> ArrayMap<K, V, N> {
    pub fn new() -> Self {
        Self {
            table_: array::from_fn(|_| None),
        }
    }

    pub fn Get(&self, key: &K) -> Option<V> {
        self.table_
            .iter()
            .flatten()
            .find(|(stored, _)| stored == key)
            .map(|(_, value)| *value)
    }

    pub fn Put(&mut self, key: &K, value: &V) {
        if let Some(slot) = self.table_.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some((*key, *value));
        }
    }

    pub fn Delete(&mut self, key: &K) {
        if let Some(slot) = self
            .table_
            .iter_mut()
            .find(|slot| slot.as_ref().is_some_and(|(stored, _)| stored == key))
        {
            *slot = None;
        }
    }
}

impl<K: Copy + PartialEq, V: Copy, const N: usize> Default for ArrayMap<K, V, N> {
    fn default() -> Self {
        Self::new()
    }
}
