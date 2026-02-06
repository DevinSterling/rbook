//! Archive `write` module that enables adding, removing,
//! and relocating content in a [`ResourceArchive`].

use crate::ebook::archive::{ResourceArchive, empty};
use crate::ebook::errors::{ArchiveError, ArchiveResult};
use crate::ebook::resource::{Resource, ResourceContent, ResourceKey};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::{fs, io};

pub(super) type ArchiveOverlay = HashMap<ArchiveResourceKey, OverlayResource>;
pub(super) type ResourceKeySet<'a> = HashSet<Cow<'a, ResourceKey<'a>>>;

impl ResourceArchive {
    pub(crate) fn empty() -> Self {
        Self::new(Box::new(empty::EmptyArchive))
    }

    pub(crate) fn remove(&mut self, key: &ResourceKey) -> Option<ResourceContent> {
        self.overlay
            .remove(key)
            .and_then(OverlayResource::take_content)
    }

    pub(crate) fn insert(
        &mut self,
        key: ResourceKey,
        content: ResourceContent,
    ) -> Option<ResourceContent> {
        let key = ArchiveResourceKey::new(key);

        self.overlay
            .insert(key, OverlayResource::Content(content))
            .and_then(OverlayResource::take_content)
    }

    /// Values are percent-decoded and the prefix `/` is stripped.
    pub(crate) fn relocate<'a>(&mut self, current: ResourceKey<'a>, new_key: ResourceKey<'a>) {
        // `current` doesn't need to be an `ArchiveResourceKey` here.
        // (reduces unnecessary heap allocation)
        let new_key = ArchiveResourceKey::new(new_key);

        if let Some(entry) = self.overlay.remove(&current) {
            // Case 1: In Memory        -> Move bytes to new key
            // Case 2: Relocated Before -> Move relocation pointer to new key
            self.overlay.insert(new_key, entry);
        } else {
            // Case 3: Content must be in `base` at `current` -> Create relocation pointer
            let current = ArchiveResourceKey::new(current);

            self.overlay
                .insert(new_key, OverlayResource::Relocated(current.0));
        }
    }

    pub(crate) fn is_overlay_resource(&self, key: &ResourceKey<'_>) -> bool {
        self.overlay.contains_key(key)
    }
}

/// This helper allows convenient [lookup](HashMap::get) rom `&mut HashMap<K, V>`
/// without requiring [`ResourceKey`] to have a `'static` lifetime.
#[derive(Debug, Hash, PartialEq, Eq)]
pub(super) struct ArchiveResourceKey(pub(super) ResourceKey<'static>);

impl ArchiveResourceKey {
    fn new(key: ResourceKey<'_>) -> Self {
        Self(match key {
            ResourceKey::Value(value) => ResourceKey::Value(Cow::Owned(value.into_owned())),
            ResourceKey::Position(position) => ResourceKey::Position(position),
        })
    }
}

impl<'a> std::borrow::Borrow<ResourceKey<'a>> for ArchiveResourceKey {
    fn borrow(&self) -> &ResourceKey<'a> {
        &self.0
    }
}

#[derive(Debug)]
pub(super) enum OverlayResource {
    /// New or overwritten content.
    Content(ResourceContent),
    /// Content within the `base` archive that has been relocated.
    Relocated(ResourceKey<'static>),
}

impl OverlayResource {
    fn take_content(self) -> Option<ResourceContent> {
        match self {
            Self::Content(data) => Some(data),
            Self::Relocated(_) => None,
        }
    }
}

impl ResourceContent {
    pub(super) fn copy_bytes<W: Write>(
        &self,
        resource: &Resource<'_>,
        mut writer: W,
    ) -> ArchiveResult<u64> {
        match self {
            ResourceContent::Memory(content) => {
                writer.write_all(content).map(|_| content.len() as u64)
            }
            ResourceContent::File(path) => {
                fs::File::open(path).and_then(|mut file| io::copy(&mut file, &mut writer))
            }
        }
        .map_err(|source| ArchiveError::CannotRead {
            resource: resource.as_static(),
            source,
        })
    }
}
