use std::borrow::Borrow;

pub(crate) trait IteratorExt: Iterator {
    fn contains<Q>(&mut self, target: &Q) -> bool
    where
        Self: Sized,
        Self::Item: Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
        self.any(|item| item.borrow() == target)
    }

    fn one_or_many(mut self) -> OneOrMany<impl Iterator<Item = Self::Item>>
    where
        Self: Sized,
    {
        match self.next() {
            None => OneOrMany::None,
            Some(one) => match self.next() {
                None => OneOrMany::One(one),
                Some(two) => OneOrMany::Many([one, two].into_iter().chain(self)),
            },
        }
    }

    fn repeatable(self) -> Repeatable<Self::Item>
    where
        Self: Sized,
    {
        Repeatable::new(self)
    }
}

impl<I: Iterator> IteratorExt for I {}

pub(crate) enum OneOrMany<I: IntoIterator> {
    None,
    One(I::Item),
    Many(I),
}

/// A helper factory that creates repeatable iterators.
pub(crate) struct Repeatable<T>(OneOrMany<Vec<T>>);

impl<T> Repeatable<T> {
    fn new(iterator: impl Iterator<Item = T>) -> Self {
        match iterator.one_or_many() {
            OneOrMany::None => Self(OneOrMany::None),
            OneOrMany::One(entry) => Self(OneOrMany::One(entry)),
            OneOrMany::Many(many) => Self(OneOrMany::Many(many.collect())),
        }
    }

    pub(crate) fn repeat_once(&self) -> RepeatedIter<'_, T> {
        self.repeat(1)
    }

    pub(crate) fn repeat(&self, n: usize) -> RepeatedIter<'_, T> {
        RepeatedIter {
            count: n,
            index: 0,
            repeated: self,
        }
    }
}

pub(crate) struct RepeatedIter<'a, T> {
    count: usize,
    index: usize,
    repeated: &'a Repeatable<T>,
}

impl<'a, T> Iterator for RepeatedIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }

        match &self.repeated.0 {
            OneOrMany::None => {
                self.count = 0;
                None
            }
            OneOrMany::One(item) => {
                self.count -= 1;
                Some(item)
            }
            OneOrMany::Many(items) => {
                let item = items.get(self.index);
                self.index = (self.index + 1) % items.len();

                if self.index == 0 {
                    self.count -= 1;
                }
                item
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct IndexCursor {
    /// `index` is always less than `len`
    index: Option<usize>,
    len: usize,
}

impl IndexCursor {
    pub(crate) fn new(len: usize) -> Self {
        Self { index: None, len }
    }

    pub(crate) fn reset(&mut self) {
        self.index = None;
    }

    pub(crate) fn set(&mut self, index: usize) {
        self.index = Some(self.len.min(index));
    }

    /// Increments and returns the new index.
    /// Returns [`None`] when the index cannot be incremented.
    pub(crate) fn increment(&mut self) -> Option<usize> {
        self.index = Some(match self.index {
            Some(index) if index + 1 < self.len => index + 1,
            None if self.len > 0 => 0,
            _ => return None,
        });
        self.index
    }

    /// Decrements and returns the new index.
    /// Returns [`None`] when the index cannot be decremented.
    pub(crate) fn decrement(&mut self) -> Option<usize> {
        self.index = Some(match self.index {
            Some(index) if index > 0 => index - 1,
            _ => return None,
        });
        self.index
    }

    pub(crate) fn index(&self) -> Option<usize> {
        self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_or_many() {
        let iter_none = std::iter::empty::<u32>();
        assert!(matches!(iter_none.one_or_many(), OneOrMany::None));

        let iter_one = [1].into_iter();
        match iter_one.one_or_many() {
            OneOrMany::One(val) => assert_eq!(1, val),
            _ => panic!("Expected OneOrMany::One"),
        }

        let many = [vec![1, 2], vec![3, 4, 5, 6]];
        for case in many {
            match case.clone().into_iter().one_or_many() {
                OneOrMany::Many(iter) => {
                    assert_eq!(case, iter.collect::<Vec<_>>());
                }
                _ => panic!("Expected OneOrMany::Many"),
            }
        }
    }

    #[test]
    fn test_repeatable_empty() {
        let iter_none = std::iter::empty::<i32>();

        let repeatable = iter_none.repeatable();
        assert_eq!(0, repeatable.repeat_once().count());
        assert_eq!(0, repeatable.repeat(5).count());

        let mut exhausted = repeatable.repeat_once();
        assert_eq!(None, exhausted.next());
        assert_eq!(None, exhausted.next());
    }

    #[test]
    fn test_repeatable_single() {
        let repeatable = [42].into_iter().repeatable();

        let zero: Vec<_> = repeatable.repeat(0).copied().collect();
        assert_eq!(zero, vec![]);

        let once: Vec<_> = repeatable.repeat_once().copied().collect();
        assert_eq!(vec![42], once);

        let thrice: Vec<_> = repeatable.repeat(3).copied().collect();
        assert_eq!(vec![42, 42, 42], thrice);

        let mut exhausted = repeatable.repeat_once();
        assert_eq!(Some(&42), exhausted.next());
        assert_eq!(None, exhausted.next());
        assert_eq!(None, exhausted.next());
    }

    #[test]
    fn test_repeatable_many() {
        let repeatable = [1, 2, 3].into_iter().repeatable();

        let zero: Vec<_> = repeatable.repeat(0).copied().collect();
        assert_eq!(zero, vec![]);

        let once: Vec<_> = repeatable.repeat_once().copied().collect();
        assert_eq!(vec![1, 2, 3], once);

        let twice: Vec<_> = repeatable.repeat(2).copied().collect();
        assert_eq!(vec![1, 2, 3, 1, 2, 3], twice);

        let mut exhausted = repeatable.repeat_once();
        assert_eq!(exhausted.next(), Some(&1));
        assert_eq!(exhausted.next(), Some(&2));
        assert_eq!(exhausted.next(), Some(&3));
        assert_eq!(exhausted.next(), None);
        assert_eq!(exhausted.next(), None);
    }
}
