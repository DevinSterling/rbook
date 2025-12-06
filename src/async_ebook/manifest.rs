//! todo

use crate::ebook::errors::EbookResult;
use crate::ebook::manifest::ManifestEntry;

/// todo
pub trait AsyncManifestEntry<'ebook>: ManifestEntry<'ebook> {
    /// todo
    fn read_str(&self) -> impl Future<Output = EbookResult<String>> + Send;

    /// todo
    fn read_bytes(&self) -> impl Future<Output = EbookResult<Vec<u8>>> + Send;
}
