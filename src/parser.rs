pub(crate) mod xml;

use crate::ebook::errors::FormatError;

pub(crate) type ParserResult<T> = Result<T, FormatError>;
