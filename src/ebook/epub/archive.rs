use crate::ebook::archive::{Archive, ResourceArchive};
use crate::ebook::errors::ArchiveResult;
use crate::ebook::resource::{Resource, ResourceKey};
use crate::util::uri;
use std::io::Write;

#[cfg(feature = "write")]
use {
    crate::ebook::resource::ResourceContent, crate::util::borrow::CowExt, std::borrow::Cow,
    std::collections::HashSet,
};

#[derive(Debug)]
pub(super) struct EpubArchive(ResourceArchive);

impl EpubArchive {
    pub(super) fn new(base: Box<dyn Archive>) -> Self {
        Self(ResourceArchive::new(base))
    }

    /// Avoids double-decoding the given `resource` key, as it is already percent-decoded.
    pub(super) fn copy_resource_decoded(
        &self,
        resource: &Resource,
        writer: &mut dyn Write,
    ) -> ArchiveResult<u64> {
        // Since the given resource key is already decoded,
        // calling `transform_href` is not needed.
        self.0.copy_resource(resource, writer)
    }

    fn transform_href(href: &str) -> ResourceKey<'_> {
        // If `is_encoded` is `true`, the given href is not decoded.
        // - This avoids double-decoding
        //
        // Unlike `transform_owned_href`, this method does not add a `/` prefix.
        // Adding a prefix is not required as paths given to this method are already
        // prefixed with `/`.
        ResourceKey::Value(uri::decode(href))
    }
}

#[cfg(feature = "write")]
impl EpubArchive {
    pub(super) fn empty() -> Self {
        Self(ResourceArchive::empty())
    }

    pub(super) fn remove(&mut self, href: &str) -> Option<ResourceContent> {
        self.0.remove(&Self::transform_href(href))
    }

    pub(super) fn insert(
        &mut self,
        href: String,
        content: ResourceContent,
    ) -> Option<ResourceContent> {
        self.0.insert(Self::transform_owned_href(href), content)
    }

    /// Values are percent-decoded
    pub(super) fn relocate(&mut self, current: impl Into<String>, new_href: impl Into<String>) {
        self.0.relocate(
            Self::transform_owned_href(current.into()),
            Self::transform_owned_href(new_href.into()),
        )
    }

    /// Checks if the given `href` points to an overlay resource.
    ///
    /// The given `href` is percent-decoded.
    pub(super) fn is_overlay_resource(&self, href: &str) -> bool {
        self.0.is_overlay_resource(&ResourceKey::from(href))
    }

    fn transform_owned_href(href: String) -> ResourceKey<'static> {
        let decoded = uri::decode(&href).take_owned().unwrap_or(href);

        // For consistency, ensure all given hrefs are prefixed with `/`,
        // indicating the EPUB container root.
        ResourceKey::Value(Cow::Owned(uri::into_absolute(decoded)))
    }
}

impl Archive for EpubArchive {
    fn copy_resource(&self, resource: &Resource, writer: &mut dyn Write) -> ArchiveResult<u64> {
        // Ensure the given resource key value is decoded
        let transformed = match resource.key() {
            ResourceKey::Value(href) => Self::transform_href(href),
            ResourceKey::Position(position) => ResourceKey::Position(*position),
        };

        self.0.copy_resource(&Resource::from(transformed), writer)
    }

    /// [Paths](ResourceKey::Value) are prefixed with `/`, indicating the EPUB container root.
    ///
    /// All contained resources are percent-decoded.
    #[cfg(feature = "write")]
    fn resources(&self) -> ArchiveResult<HashSet<Cow<'_, ResourceKey<'_>>>> {
        self.0.resources()
    }
}
