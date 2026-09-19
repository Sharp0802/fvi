use std::hash::Hash;
use std::iter::FusedIterator;

use crate::cache::{Cache, ItemLoc};

#[derive(Debug)]
pub struct Sweep<'a, K: Eq + Hash, V> {
    cache: &'a mut Cache<K, V>,
    segment: usize,
    segment_len: usize,
    item: usize,
}

impl<'a, K: Eq + Hash, V> Sweep<'a, K, V> {
    pub const fn new(cache: &'a mut Cache<K, V>, segment_len: usize) -> Self {
        Self {
            cache,
            segment: 0,
            segment_len,
            item: 0,
        }
    }
}

impl<K: Eq + Hash, V> Sweep<'_, K, V> {
    fn next_within(&mut self) -> Option<(K, V)> {
        let now = self.cache.now;

        while let Some(item) = self.cache.segments[self.segment].items.get(self.item) {
            let loc = ItemLoc {
                segment: self.segment as u32,
                item: self.item as u32,
            };

            if item.epoch != now {
                let (entry, removed) = self.cache.remove(&loc);
                entry.remove();
                return Some((removed.key, removed.value));
            }

            if self.segment == self.cache.segments.len() - 1 {
                self.item += 1;
                continue;
            }

            let (entry, removed) = self.cache.remove(&loc);
            let bucket = entry.bucket_index();

            let items = &mut self.cache.segments[self.segment + 1].items;
            let index = items.len();
            items.push(removed);

            let entry = self.cache.table.get_bucket_mut(bucket).unwrap();
            entry.loc.segment = (self.segment + 1) as u32;
            entry.loc.item = index as u32;
        }

        None
    }
}

impl<K: Eq + Hash, V> Iterator for Sweep<'_, K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        while self.segment < self.segment_len {
            let item = self.next_within();
            if let Some(item) = item {
                return Some(item);
            }

            self.segment += 1;
            self.item = 0;
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.segment >= self.segment_len {
            return (0, Some(0));
        }

        let rem = self.cache.segments[self.segment..self.segment_len]
            .iter()
            .map(|segment| segment.items.len())
            .sum::<usize>()
            - self.item;

        (0, Some(rem))
    }
}

impl<K: Eq + Hash, V> FusedIterator for Sweep<'_, K, V> {}

impl<K: Eq + Hash, V> Drop for Sweep<'_, K, V> {
    fn drop(&mut self) {
        debug_assert!(self.next().is_none());
        self.cache.adapt();
    }
}
