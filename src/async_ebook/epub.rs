//! todo

use crate::async_ebook::AsyncEbook;
use crate::async_ebook::archive::AsyncArchive;
use crate::async_ebook::manifest::AsyncManifestEntry;
use crate::ebook::epub::manifest::EpubManifestEntryData;
use crate::ebook::epub::{ArchiveLike, EpubData, EpubResourceProvider};
use crate::ebook::errors::EbookResult;
use crate::ebook::manifest::ManifestEntry;
use crate::ebook::resource::Resource;

/// todo
pub type AsyncEpub = EpubData<AsynchronousArchive>;
/// todo
pub type AsyncEpubManifestEntry<'ebook> =
    EpubManifestEntryData<'ebook, &'ebook AsynchronousArchive>;

// todo: change location
/// todo
pub struct AsynchronousArchive(Box<dyn AsyncArchive>);

impl ArchiveLike for AsynchronousArchive {
    type Item<'a> = &'a AsynchronousArchive;

    fn as_ref(&self) -> Self::Item<'_> {
        self
    }
}

impl AsyncEbook for AsyncEpub {
    async fn read_resource_str<'a>(
        &self,
        resource: impl Into<Resource<'a>>,
    ) -> EbookResult<String> {
        self.archive
            .0
            .read_resource_str(&self.transform_resource(resource.into()))
            .await
            .map_err(Into::into)
    }

    async fn read_resource_bytes<'a>(
        &self,
        resource: impl Into<Resource<'a>>,
    ) -> EbookResult<Vec<u8>> {
        self.archive
            .0
            .read_resource_bytes(&self.transform_resource(resource.into()))
            .await
            .map_err(Into::into)
    }
}

impl<'ebook> EpubResourceProvider<'ebook, &'ebook AsynchronousArchive> {
    pub(crate) async fn read_str<'a>(&self, resource: Resource<'a>) -> EbookResult<String> {
        self.0
            .0
            .read_resource_str(&resource)
            .await
            .map_err(Into::into)
    }

    pub(crate) async fn read_bytes<'a>(&self, resource: Resource<'a>) -> EbookResult<Vec<u8>> {
        self.0
            .0
            .read_resource_bytes(&resource)
            .await
            .map_err(Into::into)
    }
}

impl<'ebook> AsyncManifestEntry<'ebook> for AsyncEpubManifestEntry<'ebook> {
    async fn read_str(&self) -> EbookResult<String> {
        self.context.resource.read_str(self.resource()).await
    }

    async fn read_bytes(&self) -> EbookResult<Vec<u8>> {
        self.context.resource.read_bytes(self.resource()).await
    }
}
