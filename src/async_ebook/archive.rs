use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::resource::Resource;
use crate::util::sync::SendAndSync;

#[async_trait::async_trait]
pub(super) trait AsyncArchive: SendAndSync {
    async fn read_resource_bytes(&self, resource: &Resource) -> ArchiveResult<Vec<u8>>;

    //async fn read_resource_bytes_utf8(&self, resource: &Resource) -> ArchiveResult<Vec<u8>>;

    async fn read_resource_str(&self, resource: &Resource) -> ArchiveResult<String>;
}
