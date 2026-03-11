macro_rules! inherent {
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
pub(crate) use inherent;
