use std::collections::HashMap;

/// A small in-memory cache with TTL support.
pub struct Cache<K, V> {
    entries: HashMap<K, (V, u64)>,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> Cache<K, V> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V, ttl: u64) {
        self.entries.insert(key, (value, ttl));
    }

    pub fn get(&self, key: &K, now: u64) -> Option<V> {
        self.entries.get(key).and_then(|(v, ttl)| {
            if now <= *ttl {
                Some(v.clone())
            } else {
                None
            }
        })
    }

    pub fn evict_expired(&mut self, now: u64) {
        self.entries.retain(|_, (_, ttl)| *ttl >= now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_roundtrip() {
        let mut c = Cache::new();
        c.insert("a", 1, 100);
        assert_eq!(c.get(&"a", 50), Some(1));
        assert_eq!(c.get(&"a", 150), None);
    }
}
