use hashbrown::hash_table::{Entry, OccupiedEntry};
use hashbrown::{DefaultHashBuilder, HashTable};
use std::hash::{BuildHasher, Hash};

use crate::cache::sweep::Sweep;

pub mod sweep;

#[derive(Debug)]
struct Item<K, V> {
    epoch: u32,
    key: K,
    value: V,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ItemLoc {
    segment: u32,
    item: u32,
}

#[derive(Clone, Copy, Debug)]
struct TableEntry {
    loc: ItemLoc,
    hash: u64,
}

#[derive(Debug)]
struct Segment<K, V> {
    max_len: usize,
    items: Vec<Item<K, V>>,
}

impl<K, V> Segment<K, V> {
    pub const fn new(max_len: usize) -> Self {
        Self {
            max_len,
            items: Vec::new(),
        }
    }

    pub const fn is_sweep_required(&self) -> bool {
        self.items.len() > self.max_len
    }
}

#[derive(Debug)]
pub struct Cache<K, V> {
    now: u32,
    segments: Vec<Segment<K, V>>,
    table: HashTable<TableEntry>,
    hasher: DefaultHashBuilder,
}

impl<K: Eq + Hash, V> Cache<K, V> {
    pub fn new() -> Self {
        Self {
            now: 0,
            #[rustfmt::skip]
            segments: vec![
                Segment::new(512),
                Segment::new(128),
                Segment::new(64),
            ],
            table: HashTable::new(),
            hasher: DefaultHashBuilder::default(),
        }
    }

    pub fn clear(&mut self) {
        self.segments.clear();
        self.table.clear();
    }

    fn try_fetch_entry<E>(
        &mut self,
        key: K,
        default: impl FnOnce(&K) -> Result<V, E>,
    ) -> Result<TableEntry, E> {
        let hash = self.hasher.hash_one(&key);

        let entry = self.table.entry(
            hash,
            |&other| {
                let segment = &self.segments[other.loc.segment as usize];
                segment.items[other.loc.item as usize].key == key
            },
            |old| old.hash,
        );

        let &entry = match entry {
            Entry::Occupied(entry) => entry,
            Entry::Vacant(vacant) => {
                let value = default(&key)?;

                let index = self.segments[0].items.len();
                self.segments[0].items.push(Item {
                    epoch: self.now,
                    key,
                    value,
                });

                vacant.insert(TableEntry {
                    loc: ItemLoc {
                        segment: 0,
                        item: index.try_into().expect("insufficient address range"),
                    },
                    hash,
                })
            }
        }
        .get();

        self.segments[entry.loc.segment as usize].items[entry.loc.item as usize].epoch = self.now;

        Ok(entry)
    }

    fn fetch_entry(&mut self, key: K, default: impl FnOnce(&K) -> V) -> TableEntry {
        let hash = self.hasher.hash_one(&key);

        let &entry = self
            .table
            .entry(
                hash,
                |&other| {
                    let segment = &self.segments[other.loc.segment as usize];
                    segment.items[other.loc.item as usize].key == key
                },
                |old| old.hash,
            )
            .or_insert_with(|| {
                let value = default(&key);

                let index = self.segments[0].items.len();
                self.segments[0].items.push(Item {
                    epoch: self.now,
                    key,
                    value,
                });

                TableEntry {
                    loc: ItemLoc {
                        segment: 0,
                        item: index.try_into().expect("insufficient address range"),
                    },
                    hash,
                }
            })
            .get();

        self.segments[entry.loc.segment as usize].items[entry.loc.item as usize].epoch = self.now;

        entry
    }

    fn remove(&mut self, loc: ItemLoc) -> (OccupiedEntry<'_, TableEntry>, Item<K, V>) {
        let segment = loc.segment as usize;
        let index = loc.item as usize;

        let removed_hash = {
            let item = &self.segments[segment].items[index];
            self.hasher.hash_one(&item.key)
        };

        let removed_bucket = self
            .table
            .find_bucket_index(removed_hash, |entry| entry.loc == loc)
            .unwrap();

        let items = &mut self.segments[segment].items;
        let removed = items.swap_remove(index);

        if let Some(moved) = items.get(index) {
            let old_loc = ItemLoc {
                segment: loc.segment,
                item: items.len().try_into().expect("insufficient address range"),
            };

            let moved_hash = self.hasher.hash_one(&moved.key);

            let moved_entry = self
                .table
                .find_mut(moved_hash, |entry| entry.loc == old_loc)
                .unwrap();

            moved_entry.loc = loc;
        }

        let occupied = self.table.get_bucket_entry(removed_bucket).unwrap();

        (occupied, removed)
    }

    pub fn try_fetch<E>(
        &mut self,
        key: K,
        default: impl FnOnce(&K) -> Result<V, E>,
    ) -> Result<&V, E> {
        let entry = self.try_fetch_entry(key, default)?;
        Ok(&self.segments[entry.loc.segment as usize].items[entry.loc.item as usize].value)
    }

    pub fn fetch(&mut self, key: K, default: impl FnOnce(&K) -> V) -> &V {
        let entry = self.fetch_entry(key, default);
        &self.segments[entry.loc.segment as usize].items[entry.loc.item as usize].value
    }

    pub const fn tick(&mut self) {
        self.now = self.now.wrapping_add(1);
    }

    pub fn sweep(&mut self) -> Sweep<'_, K, V> {
        for (i, segment) in self.segments.iter().enumerate().rev() {
            if segment.is_sweep_required() {
                return Sweep::new(self, i + 1);
            }
        }

        Sweep::new(self, 0)
    }

    fn adapt(&mut self) {
        for segment in &mut self.segments {
            let len = segment.items.len();
            let capacity = segment.max_len;

            segment.max_len = if len <= capacity / 2 {
                capacity / 2 + 1
            } else if len > capacity {
                len.max(capacity + capacity / 2)
            } else {
                capacity
            };
        }
    }
}
