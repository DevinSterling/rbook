#[cfg(feature = "write")]
pub(crate) trait IteratorExt {
    fn has_one_remaining(&self) -> bool;
}

#[cfg(feature = "write")]
impl<I: Iterator> IteratorExt for I {
    fn has_one_remaining(&self) -> bool {
        matches!(self.size_hint(), (1, Some(1)))
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
