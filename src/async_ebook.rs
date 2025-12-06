//! todo

mod archive;
pub mod epub;
pub mod manifest;

use crate::ebook::Ebook;
use crate::ebook::errors::EbookResult;
use crate::ebook::resource::Resource;

/// todo
pub trait AsyncEbook: Ebook {
    /// todo
    fn read_resource_str<'a>(
        &self,
        resource: impl Into<Resource<'a>> + Send,
    ) -> impl Future<Output = EbookResult<String>> + Send;

    /// todo
    fn read_resource_bytes<'a>(
        &self,
        resource: impl Into<Resource<'a>> + Send,
    ) -> impl Future<Output = EbookResult<Vec<u8>>> + Send;
}
