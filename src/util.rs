pub(crate) mod uri;
pub(crate) mod utf8;

#[cfg(feature = "threadsafe")]
pub(crate) mod sync {
    pub(crate) use std::sync::Arc as Shared;
    pub(crate) use std::sync::Mutex as Lock;
    /// Marker to identify an implementing instance as thread-safe.
    pub(crate) trait SendAndSync: Send + Sync {}
}

#[cfg(not(feature = "threadsafe"))]
pub(crate) mod sync {
    pub(crate) use std::cell::RefCell as Lock;
    pub(crate) use std::rc::Rc as Shared;
    pub(crate) trait SendAndSync {}
}

impl<#[cfg(feature = "threadsafe")] A: Send + Sync, #[cfg(not(feature = "threadsafe"))] A>
    sync::SendAndSync for A
{
}

pub(crate) trait StringExt {
    fn trim_in_place(&mut self);
}

impl StringExt for String {
    fn trim_in_place(&mut self) {
        let trim_end = self.trim_end();
        self.truncate(trim_end.len());

        let trim_start = self.trim_start();
        self.replace_range(..(self.len() - trim_start.len()), "");

        self.shrink_to_fit();
    }
}

pub(crate) trait StrExt {
    fn starts_with_ignore_case(&self, start: &str) -> bool;
}

impl StrExt for &str {
    fn starts_with_ignore_case(&self, start: &str) -> bool {
        self.len() >= start.len() && self[..start.len()].eq_ignore_ascii_case(start)
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
            None => 0,
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
