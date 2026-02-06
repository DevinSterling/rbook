use crate::ebook::archive::Archive;
use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::errors::ArchiveError;
use crate::ebook::resource::Resource;
use std::io;
use std::io::Write;

#[cfg(feature = "write")]
use crate::ebook::archive::write::ResourceKeySet;

pub(crate) struct EmptyArchive;

impl Archive for EmptyArchive {
    fn copy_resource(&self, resource: &Resource, _writer: &mut dyn Write) -> ArchiveResult<u64> {
        Err(ArchiveError::InvalidResource {
            source: io::Error::new(io::ErrorKind::NotFound, "Requested resource does not exist"),
            resource: resource.as_static(),
        })
    }

    #[cfg(feature = "write")]
    fn resources(&self) -> ArchiveResult<ResourceKeySet<'_>> {
        Ok(ResourceKeySet::new())
    }
}
