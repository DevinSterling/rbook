use crate::ebook::archive::Archive;
use crate::ebook::element::Href;
use crate::ebook::epub::write::writer::{EpubWriter, EpubWriterContext};
use crate::ebook::resource::{Resource, ResourceKey};
use crate::util::uri;
use crate::writer::WriterResult;
use crate::writer::zip::ZipWriter;
use std::borrow::Cow;
use std::collections::HashSet;
use std::io::Write;

/// Writes all non-generated EPUB resources.
///
/// # Unwritten Resources
/// **Not written here (as it's generated)**:
/// - Package file
/// - Container file (META-INF/container.xml)
/// - Mimetype file (mimetype)
///
/// **Conditionally written here (if toc generation is disabled)**:
/// - Toc navigation files (EPUB 2 NCX, EPUB 3 NAV)
///
/// # Note
/// - All hrefs/file paths retrieved from the underlying `Epub` must be percent-decoded.
/// - `archive` and `resources` contain undecoded paths.
struct ResourceWriter<'ebook, W: Write> {
    ctx: &'ebook EpubWriterContext<'ebook>,
    // New Zip archive to write to
    zip: &'ebook mut ZipWriter<W>,
    // All resource contained within the original Zip archive
    resources: HashSet<Cow<'ebook, ResourceKey<'ebook>>>,
}

impl<'ebook, W: Write> ResourceWriter<'ebook, W> {
    fn new(
        ctx: &'ebook EpubWriterContext<'ebook>,
        zip: &'ebook mut ZipWriter<W>,
        resources: HashSet<Cow<'ebook, ResourceKey<'ebook>>>,
    ) -> Self {
        Self {
            ctx,
            zip,
            resources,
        }
    }

    fn remove_resource(&mut self, resource: impl Into<ResourceKey<'ebook>>) {
        self.resources.remove(&Cow::Owned(resource.into()));
    }

    fn write_resources(mut self) -> WriterResult<()> {
        const CONTAINER_FILE: &str = "/META-INF/container.xml";
        const MIMETYPE: &str = "/mimetype";

        // AVOID writing here as they are written manually elsewhere
        // - Package file           (Written manually @ EpubWriter::write_package)
        // - META-INF/container.xml (Written manually @ EpubWriter::write_container)
        // - mimetype               (Written manually @ EpubWriter::write_mimetype)
        let package_file = uri::decode(&self.ctx.epub.package.location);
        self.remove_resource(package_file);
        self.remove_resource(CONTAINER_FILE);
        self.remove_resource(MIMETYPE);

        self.write_manifest_resources()?;
        self.write_orphaned_resources()
    }

    fn write_manifest_resources(&mut self) -> WriterResult<()> {
        let archive = &self.ctx.epub.archive;

        // If generated, avoid writing as it's written manually elsewhere
        // - ToC-related resources  (Written manually @ EpubWriter::write_toc)
        let skip_id = |id| {
            let ncx = self.ctx.toc.epub2_ncx.as_ref();
            let nav = self.ctx.toc.epub3_nav.as_ref();

            self.ctx.config.generate_toc
                && (ncx.is_some_and(|ncx| !ncx.is_generated && id == ncx.id)
                    || nav.is_some_and(|nav| !nav.is_generated && id == nav.id))
        };

        for (id, entry) in &self.ctx.epub.manifest.entries {
            let decoded = uri::decode(&entry.href);
            let path = &*decoded;

            if !skip_id(id.as_str()) {
                self.zip.start_file(path)?;
                // `copy_resource_decoded` is called here as
                // `path` is percent-decoded from `uri::decode`
                archive.copy_resource_decoded(&Resource::from(path), &mut self.zip)?;
            }
            self.resources
                .remove(&Cow::Owned(ResourceKey::from(decoded)));
        }
        Ok(())
    }

    fn write_orphaned_resources(&mut self) -> WriterResult<()> {
        const META_INF_DIRECTORY: &str = "/META-INF/";

        let archive = &self.ctx.epub.archive;
        let retain = |path| match &self.ctx.config.keep_orphans {
            Some(filter) => filter.filter(Href::new(path)),
            // Files in `/META-INF/`
            None => path.starts_with(META_INF_DIRECTORY),
        };

        for resource in &self.resources {
            let Some(path) = resource.value() else {
                continue;
            };

            // Keep the file if it explicitly provided by the user (An "overlay" resource)
            if archive.is_overlay_resource(path) || retain(path) {
                // Add file to zip
                self.zip.start_file(path)?;
                // `copy_resource_decoded` is called here as each and every
                // `resource` given directly from an archive is always decoded.
                archive.copy_resource_decoded(&path.into(), &mut self.zip)?;
            }
        }
        Ok(())
    }
}

impl<W: Write> EpubWriter<'_, W> {
    pub(super) fn write_resources(&mut self) -> WriterResult<()> {
        let resources = self.ctx.epub.archive.resources()?;
        ResourceWriter::new(&self.ctx, &mut self.zip, resources).write_resources()
    }
}
