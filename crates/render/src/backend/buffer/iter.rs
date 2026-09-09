use std::iter::{FusedIterator, Peekable};
use std::marker::PhantomData;
use std::ops::Range;

use super::{MAX_BATCH_BYTES, MAX_HOLE_BYTES};

pub struct CoarseIter<I: Iterator, T> {
    iter: Peekable<I>,
    _marker: PhantomData<fn() -> T>,
}

impl<I, T> From<I> for CoarseIter<I, T>
where
    I: Iterator,
{
    fn from(value: I) -> Self {
        Self {
            iter: value.peekable(),
            _marker: PhantomData,
        }
    }
}

impl<I, T> Iterator for CoarseIter<I, T>
where
    I: Iterator<Item = usize>,
{
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let max_hole_size: usize = MAX_HOLE_BYTES / size_of::<T>();
        let max_batch_size: usize = (MAX_BATCH_BYTES / size_of::<T>()).max(1);

        let start = self.iter.next()?;
        let mut end = start + 1;

        while let Some(&next) = self.iter.peek() {
            if next - end > max_hole_size || next - start + 1 > max_batch_size {
                break;
            }

            end = next + 1;
            self.iter.next();
        }

        Some(start..end)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.iter.size_hint();
        (min.min(1), max)
    }
}

impl<I, T> FusedIterator for CoarseIter<I, T>
where
    Self: Iterator,
    I: FusedIterator,
{
}
