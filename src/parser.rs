pub(crate) mod xml;

pub(crate) type ParserResult<T> = Result<T, crate::ebook::errors::FormatError>;
