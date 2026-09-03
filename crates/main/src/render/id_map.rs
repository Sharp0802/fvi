use std::collections::HashMap;
use std::num::NonZero;

use crate::core::collections::List;
use crate::render::Id;

/// A sparse-to-dense mapping.
#[derive(Clone, Debug)]
pub struct IdMap {
    now: u32,
    /// The maximum tick count in unused state.
    /// Cache will be invalidated after this tick count.
    pub max_age: NonZero<u32>,
    map: HashMap<Id, usize>,
    list: List<(Id, u32)>,
}

impl IdMap {
    /// Creates a new [`IdMap`].
    #[must_use]
    pub fn new(max_age: NonZero<u32>) -> Self {
        Self {
            now: 0,
            max_age,
            map: HashMap::new(),
            list: List::new(),
        }
    }

    /// Maps given sparse identifier into
    /// the map-specifiec dense identifier.
    pub fn map(&mut self, sparse: Id) -> usize {
        if let Some(&cached) = self.map.get(&sparse) {
            self.list[cached].1 = self.now;
            self.list.set_end(cached);
            return cached;
        }

        let dense = self.list.push((sparse, self.now));
        self.map.insert(sparse, dense);
        dense
    }

    /// Advances a tick and collects outdated caches.
    pub fn tick(&mut self) {
        self.now += 1;

        while let Some((id, timestamp)) = self.list.peek()
            && self.now.wrapping_sub(*timestamp) > self.max_age.get()
        {
            self.map.remove(id);
            self.list.pop();
        }
    }
}
