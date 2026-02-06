pub(crate) trait CowExt<T: ToOwned + ?Sized> {
    fn take_owned(self) -> Option<T::Owned>;
}

impl<T: ToOwned + ?Sized> CowExt<T> for std::borrow::Cow<'_, T> {
    fn take_owned(self) -> Option<T::Owned> {
        match self {
            Self::Owned(owned) => Some(owned),
            Self::Borrowed(_) => None,
        }
    }
}

#[cfg(feature = "write")]
#[derive(Debug, PartialEq)]
pub(crate) enum MaybeOwned<'a, T> {
    Owned(T),
    Borrowed(&'a mut T),
}

#[cfg(feature = "write")]
impl<T> MaybeOwned<'_, T> {
    pub(crate) fn into_owned(self) -> Option<T> {
        match self {
            Self::Owned(owned) => Some(owned),
            Self::Borrowed(_) => None,
        }
    }
}

#[cfg(feature = "write")]
impl<'a, T> std::ops::Deref for MaybeOwned<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(owned) => owned,
            Self::Borrowed(reference) => reference,
        }
    }
}

#[cfg(feature = "write")]
impl<'a, T> std::ops::DerefMut for MaybeOwned<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Owned(owned) => owned,
            Self::Borrowed(reference) => reference,
        }
    }
}
