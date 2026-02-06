pub(crate) trait Keyed {
    type Key: ?Sized + Eq;

    fn key(&self) -> &Self::Key;
}

/// Hashmap alternative for very small collections that are typically less than 5 entries.
#[derive(Clone, Debug, Default)]
pub(crate) struct KeyedVec<K>(pub(crate) Vec<K>);

impl<K: Keyed> KeyedVec<K> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&K> {
        self.0.get(index)
    }

    pub fn by_key(&self, key: &K::Key) -> Option<&K> {
        self.0.iter().find(|entry| entry.key() == key)
    }

    pub fn has_key(&self, key: &K::Key) -> bool {
        self.0.iter().any(|entry| entry.key() == key)
    }
}

#[cfg(feature = "write")]
impl<K: Keyed> KeyedVec<K> {
    pub fn insert(&mut self, value: K) -> Option<K> {
        match self.0.iter_mut().find(|entry| entry.key() == value.key()) {
            // If the key exists, replace it
            Some(current) => Some(std::mem::replace(current, value)),
            None => {
                self.0.push(value);
                None
            }
        }
    }

    pub fn by_key_mut(&mut self, key: &K::Key) -> Option<&mut K> {
        self.0.iter_mut().find(|entry| entry.key() == key)
    }

    pub fn remove(&mut self, key: &K::Key) -> Option<K> {
        match self.0.iter().position(|entry| entry.key() == key) {
            Some(i) => Some(self.0.remove(i)),
            None => None,
        }
    }

    pub fn retain(&mut self, f: impl FnMut(&K) -> bool) {
        self.0.retain(f)
    }

    pub fn extract_if(&mut self, mut f: impl FnMut(&K) -> bool) -> impl Iterator<Item = K> {
        self.0.extract_if(.., move |entry| f(entry))
    }

    pub fn drain(&mut self) -> impl Iterator<Item = K> {
        self.0.drain(..)
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl<K: Keyed + PartialEq> PartialEq<Self> for KeyedVec<K> {
    fn eq(&self, other: &Self) -> bool {
        if self.0.len() != other.0.len() {
            return false;
        }
        // If every key in 'self' exists in 'other' with the same value, they are equal.
        self.0.iter().all(|entry| {
            // This is quick as there are typically less than 5 elements
            other
                .by_key(entry.key())
                .is_some_and(|other_entry| entry == other_entry)
        })
    }
}
