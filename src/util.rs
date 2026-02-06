pub(crate) mod borrow;
pub(crate) mod collection;
pub(crate) mod iter;
pub(crate) mod str;
pub(crate) mod sync;
pub(crate) mod uri;
pub(crate) mod utf;

pub trait Sealed {}

macro_rules! inherent_doc {
    ($trait_path:path, $trait_method:ident) => {
        concat!(
            "# Inherent\n\n",
            "See [`",
            stringify!($trait_path),
            "::",
            stringify!($trait_method),
            "`] ",
            "for trait-level details about this method. ",
            "This is a convenience inherent method so ",
            "the [`",
            stringify!($trait_path),
            "`] trait does not need to be imported.\n",
        )
    };
}
pub(crate) use inherent_doc;
