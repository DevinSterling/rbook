use crate::ebook::metadata::datetime::DateTime;
use crate::ebook::resource::ResourceContent;
use crate::ebook::resource::consts::mime;
use crate::ebook::spine::PageDirection;
use crate::ebook::toc::TocEntryKind;
use crate::epub::consts::{dc, marc, opf};
use crate::epub::manifest::DetachedEpubManifestEntry;
use crate::epub::metadata::{DetachedEpubMetaEntry, EpubMetaEntryKind, EpubVersion, marker};
use crate::epub::spine::DetachedEpubSpineEntry;
use crate::epub::toc::DetachedEpubTocEntry;
use crate::epub::{Epub, EpubWriteOptions};
use crate::input::{Batch, IntoOption, Many};
use crate::util;
use crate::util::borrow::{CowExt, MaybeOwned};
use crate::util::uri;

/// An abstraction for creating and modifying an [`Epub`],
/// accessible via [`Epub::builder`] and [`Epub::edit`].
///
/// # Editing
/// When creating a new [`Epub`], ensure all required fields are set
/// before calling [`Self::write`] to produce a spec-compliant EPUB.
///
/// ## State
/// - [`build`](Self::build) (Consumes an owned editor to return an [`Epub`])
/// - [`write`](Self::write) (Transitions to writer configuration for saving to disk or memory)
///
/// ## Container
/// - [`container_resource`](Self::container_resource)
///
/// ## Package
/// - [`package_location`](Self::package_location) (Default: `OEBPS/package.opf`)
/// - [`version`](Self::version)
///   (Default: [`EpubVersion::EPUB3`] - **backwards compatible with EPUB 2**)
///
/// ## Metadata
/// - [`identifier`](Self::identifier) ***(required)***
/// - [`title`](Self::title) ***(required)***
/// - [`language`](Self::language) ***(required)***
/// - [`publication_date`](Self::published_date)
/// - [`modified_date`](Self::modified_date)
/// - [`modified_now`](Self::modified_now)
/// - [`author`](Self::author)
/// - [`creator`](Self::creator)
/// - [`contributor`](Self::contributor)
/// - [`publisher`](Self::publisher)
/// - [`tag`](Self::tag) (subject)
/// - [`description`](Self::description)
/// - [`rights`](Self::rights)
/// - [`generator`](Self::generator) (Default: `rbook`)
/// - [`meta`](Self::meta)
/// - [`clear_meta`](Self::clear_meta)
///
/// ## Spine
/// - [`page_direction`](Self::page_direction)
///
/// ## Manifest
/// - [`cover_image`](Self::cover_image)
/// - [`resource`](Self::resource)
/// - [`chapter`](Self::chapter)
///
/// ## Toc
/// - [`toc_title`](Self::toc_title) (Default: `Table of Contents`)
/// - [`landmarks_title`](Self::landmarks_title) (Default: `Landmarks`)
/// - [`toc_stylesheet`](EpubWriteOptions::toc_stylesheet) via [`EpubWriteOptions`]
///
/// # XML Escaping
/// Text values (metadata values, chapter titles, ToC labels, and attribute values) are
/// stored as plain text (e.g. `"1 < 2 & 3"`).
/// They are XML-escaped automatically during [writing](Self::write).
///
/// See the [epub](crate::epub) module-level documentation for more details.
///
/// # See Also
/// All operations performed here are replicable using the lower-level write API,
/// except [`EpubEditor::container_resource`] (which will
/// have its associated lower-level API released in a future update).
///
/// For advanced/flexible EPUB modification, see:
/// - [`Epub::package_mut`]
/// - [`Epub::metadata_mut`]
/// - [`Epub::manifest_mut`]
/// - [`Epub::spine_mut`]
/// - [`Epub::toc_mut`]
///
/// # Examples
/// - Modifying an [`Epub`]:
/// ```no_run
/// # use rbook::Epub;
/// use rbook::epub::EpubChapter;
///
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// Epub::open("old.epub")?
///     .edit()
///     // Clearing all previous titles, subtitles, etc.
///     .clear_meta("dc:title")
///     // Appending the now sole title
///     .title("New Title")
///     // Appending a contributor
///     .contributor("Jane Doe")
///     // Appending a chapter
///     .chapter(EpubChapter::new("Chapter 1337").xhtml_body("1337"))
///     // Setting the modified date to now
///     .modified_now()
///     .write()
///     .compression(9)
///     .save("new.epub")
/// # }
/// ```
/// - Creating an [`Epub`]:
/// ```no_run
/// # use rbook::epub::metadata::{EpubVersion};
/// # use rbook::epub::{Epub, EpubChapter};
/// # use std::path::Path;
/// # const XHTML: &[u8] = &[];
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// Epub::builder()
///     .identifier("urn:doi:10.1234/abc")
///     .title("Some Story")
///     .author(["John Doe", "Jane Doe"])
///     .language("en")
///     .chapter(
///         // Standard Chapter (Auto-generates href/filename "volume_i.xhtml")
///         EpubChapter::new("Volume I").xhtml(XHTML).children([
///             EpubChapter::new("I").xhtml(XHTML),
///             // Referencing a local file stored on disk:
///             EpubChapter::new("II").xhtml(Path::new("local/external/file/c2.xhtml")),
///             // Setting href explicitly (No auto-generated href)
///             EpubChapter::new("III").href("dir/v1c3.xhtml").xhtml(XHTML),
///             // Link to a fragment in the parent file (No content provided)
///             EpubChapter::new("Section 1").href("dir/v1c3.xhtml#s1"),
///             // Hidden Resource (Added to Spine/Manifest, but hidden from ToC)
///             EpubChapter::unlisted("v1extras.xhtml").xhtml_body("<p>hi</p>"),
///         ]),
///     )
///     .cover_image(("cover.png", Path::new("local/external/file/cover.png")))
///     .write()
///     .compression(0)
///     // Save to disk or alternatively write to memory
///     .save("some_story.epub")
/// # }
/// ```
#[derive(Debug, PartialEq)]
pub struct EpubEditor<'ebook> {
    pub(super) epub: MaybeOwned<'ebook, Epub>,
}

impl EpubEditor<'static> {
    pub(super) fn new() -> Self {
        Self {
            epub: MaybeOwned::Owned(Epub::new()),
        }
    }

    /// Consumes the builder and returns the constructed [`Epub`].
    ///
    /// This method is only available on editors created via [`Epub::builder`],
    /// which own the underlying EPUB.
    /// Editors created via [`Epub::edit`] borrow an existing EPUB and
    /// cannot be consumed to create a new one.
    ///
    /// # Note
    /// The returned [`Epub`] can resume editing with an [`EpubEditor`] via [`Epub::edit`]:
    /// ```
    /// # use rbook::Epub;
    /// let mut epub = Epub::builder()
    ///     .title("An Original Story")
    ///     .build();
    ///
    /// // Resume editing using an `EpubEditor`
    /// epub.edit()
    ///     .author(["Jane Doe", "John Doe"])
    ///     .identifier("urn:doi:10.1234/abc");
    ///
    /// let metadata = epub.metadata();
    /// assert_eq!("urn:doi:10.1234/abc", metadata.identifier().unwrap().value());
    /// assert_eq!("An Original Story", metadata.title().unwrap().value());
    ///
    /// let mut creators = metadata.creators().map(|creator| creator.value());
    /// assert_eq!(Some("Jane Doe"), creators.next());
    /// assert_eq!(Some("John Doe"), creators.next());
    /// assert_eq!(None, creators.next());
    /// ```
    #[must_use]
    pub fn build(self) -> Epub {
        self.epub
            .into_owned()
            // `EpubEditor<'static>` is **never** created with `&'static Epub`
            .expect("`EpubEditor<'static>` should hold an owned `Epub`")
    }
}

impl EpubEditor<'_> {
    const UNIQUE_IDENTIFIER: &'static str = "unique-identifier";
    /// The default toc title an [`EpubEditor`] uses when generating
    /// a new table of contents.
    const DEFAULT_TOC_TITLE: &'static str = "Table of Contents";
    const DEFAULT_LANDMARKS_TITLE: &'static str = "Landmarks";

    fn generate_id(&self, base: Option<&str>) -> String {
        const BASE_MAX_LEN: usize = 50;

        let mut id = base
            .map(util::str::slugify)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| String::from("entry"));

        id.truncate(BASE_MAX_LEN);

        // Ensure the id doesn't start with a number
        if id.chars().next().is_some_and(char::is_numeric) {
            id.insert(0, '_');
        }

        self.epub.manifest.generate_unique_id(id)
    }

    fn process_manifest_entry(
        &mut self,
        resource: &mut DetachedEpubManifestEntry,
        fallback_id: Option<&str>,
    ) {
        // Replace the id if it is a placeholder
        if resource.as_view().id().is_empty() {
            let base = fallback_id.or_else(|| {
                let href = resource.as_view().href_raw().as_str();
                // Avoid href if it is empty
                (!href.is_empty()).then_some(href)
            });
            let id = self.generate_id(base);

            resource.as_mut().set_id(id);
        }
    }

    fn insert_checked_meta<I: Into<DetachedEpubMetaEntry>>(
        self,
        should_be_property: &str,
        input: impl Many<I>,
    ) -> Self {
        self.meta(Batch(
            input
                .iter_many()
                .map(|entry| entry.into().force_property(should_be_property)),
        ))
    }

    //////////////////////////////////
    // CONTAINER
    //////////////////////////////////

    /// Adds a container-level resource without adding it to the manifest.
    ///
    /// The given `location` is resolved relative to the EPUB **container root**, not the
    /// [package directory](crate::epub::package::EpubPackage::directory).
    ///
    /// This is intended for files that must exist independently of a rendition,
    /// such as OCF metadata and vendor-specific information.
    /// For standard book content (XHTML, CSS, images), see [`Self::resource`].
    ///
    /// Inserted container resources are obtainable via resource retrieval methods:
    /// - [`Epub::copy_resource`]
    /// - [`Epub::read_resource_str`]
    /// - [`Epub::read_resource_bytes`]
    ///
    /// # Reserved Paths
    /// The following paths are managed by rbook.
    /// If given to this method, they are ignored during [writing](Self::write)
    /// as they are **generated automatically**:
    /// - The OPF file path ([`EpubPackage::location`](crate::epub::EpubPackage::location))
    /// - `mimetype`
    /// - `META-INF/container.xml`
    ///
    /// # Percent Encoding
    /// The given `location` is expected to already be percent encoded.
    ///
    /// - **Malformed**: `my-file & #1.xml` (Invalid; Not percent-encoded)
    /// - Percent Encoded: `my-file%20%26%20%231.xml` (Valid; percent-encoded)
    ///
    /// # See Also
    /// - [`ResourceContent`] for details on providing data from memory (bytes/strings)
    ///   or the OS file system (paths).
    ///
    /// # Examples
    /// - Adding resources into META-INF:
    /// ```
    /// # use rbook::Epub;
    /// # const IBOOKS_XML: &[u8] = &[];
    /// # const ENCRYPTION_XML: &[u8] = &[];
    /// Epub::builder()
    ///     .container_resource("META-INF/com.apple.ibooks.display-options.xml", IBOOKS_XML)
    ///     .container_resource("META-INF/encryption.xml", ENCRYPTION_XML);
    /// ```
    pub fn container_resource(
        mut self,
        location: impl Into<String>,
        content: impl Into<ResourceContent>,
    ) -> Self {
        let location = location.into();
        let normalized = uri::normalize(&location).take_owned().unwrap_or(location);

        self.epub.archive.insert(normalized, content.into());
        self
    }

    //////////////////////////////////
    // PACKAGE
    //////////////////////////////////

    /// Sets the location of the package file.
    ///
    /// # See Also
    /// - ***[`EpubPackageMut::set_location`](crate::epub::package::EpubPackageMut::set_location)
    ///   for important details.***
    ///
    /// # Examples
    /// - Setting the package file location:
    /// ```
    /// # use rbook::Epub;
    /// let epub = Epub::builder()
    ///     // This should be the first method called if
    ///     // changing the package file location is required
    ///     .package_location("EPUB/my_package.opf")
    ///     .build();
    ///
    /// assert_eq!("/EPUB/my_package.opf", epub.package().location());
    /// ```
    pub fn package_location(mut self, location: impl Into<String>) -> Self {
        self.epub.package_mut().set_location(location);
        self
    }

    /// Sets the [`Epub`] version.
    /// By default, the version is [`EpubVersion::EPUB3`],
    /// which is backwards compatible with the legacy EPUB 2.
    ///
    /// # Note
    /// - This method is equivalent to calling
    ///   [`EpubPackageMut::set_version`](crate::epub::package::EpubPackageMut::set_version)
    ///   *without* returning the previous version.
    /// - Setting the version of an existing EPUB does not perform a conversion
    ///   (e.g., Converting EPUB 3 to EPUB 2).
    ///
    /// # Examples
    /// - Setting the version to EPUB `3.0`:
    /// ```
    /// # use rbook::Epub;
    /// use rbook::ebook::metadata::Version;
    /// use rbook::epub::metadata::EpubVersion;
    ///
    /// Epub::builder()
    ///     .version(3) // Passing an integer by major
    ///     .version(Version(3, 0))
    ///     .version(EpubVersion::EPUB3);
    /// ```
    pub fn version(mut self, version: impl Into<EpubVersion>) -> Self {
        self.epub.package_mut().set_version(version);
        self
    }

    //////////////////////////////////
    // METADATA
    //////////////////////////////////

    /// Appends one or more identifiers (`dc:identifier`) via the [`Many`] trait.
    ///
    /// When an [`Epub`] is created via [`Epub::new`] or [`Epub::builder`],
    /// the first given identifier is set as the unique identifier:
    /// ```
    /// # use rbook::Epub;
    /// let epub = Epub::builder()
    ///     .identifier("urn:doi:10.1234/abc")
    ///     .build();
    ///
    /// let unique_identifier = epub.metadata().identifier().unwrap();
    /// // If no metadata entry XML `id` was present upon insertion into
    /// // `EpubEditor::identifier`, it is set to `unique-identifier`
    /// assert_eq!(Some("unique-identifier"), unique_identifier.id());
    /// assert_eq!("urn:doi:10.1234/abc", unique_identifier.value());
    /// ```
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::identifier`] to explicitly use a builder for greater control.
    ///
    /// # Examples
    /// - Appending identifiers:
    /// ```
    /// # use rbook::Epub;
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// Epub::builder()
    ///     // Single entry
    ///     .identifier("c028b07e-477a-49e6-b17c-9ffd3b169c23")
    ///     // Batch entries
    ///     .identifier(["urn:isbn:9780000000001", "9780000000001"])
    ///     // Explicit builder
    ///     .identifier(
    ///         DetachedEpubMetaEntry::identifier("urn:doi:10.1234/abc")
    ///             .scheme("onix:codelist5", "06"),
    ///     );
    /// ```
    pub fn identifier(
        mut self,
        input: impl Many<DetachedEpubMetaEntry<marker::Identifier>>,
    ) -> Self {
        let mut iter = input.iter_many();

        // If the referenced identifier is empty, it is not set.
        // The Epub most likely was created using `Epub::new` or `Epub::builder`.
        // - Create a unique identifier using the first given metadata entry
        if self.epub.package.unique_identifier.is_empty()
            && let Some(mut identifier) = iter.next()
        {
            // Ensure an id is present
            if identifier.as_view().id().is_none() {
                identifier.as_mut().set_id(Self::UNIQUE_IDENTIFIER);
            }
            // Set the unique identifier
            if let Some(id) = identifier.as_view().id() {
                self.epub.package_mut().set_unique_identifier(id);
            }
            self.epub.metadata_mut().push(identifier);
        }
        // Insert remaining
        self.meta(Batch(iter))
    }

    /// Sets the publication date (`dc:date`).
    ///
    /// The given date is not validated.
    /// However, a date conforming to
    /// [**ISO 8601-1**](https://www.iso.org/iso-8601-date-and-time-format.html)
    /// is strongly recommended.
    ///
    /// To ensure there is exactly **one** publication date, this method finds and removes:
    /// 1. Existing plain `<dc:date>` elements.
    /// 2. Legacy `<dc:date opf:event="publication">` elements.
    ///
    /// Other dates (e.g., modification dates) are preserved.
    ///
    /// # Examples
    /// - Setting the publication date:
    /// ```
    /// use rbook::Epub;
    /// use rbook::ebook::metadata::datetime::{Date, DateTime};
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// # let iso_datetime = "";
    /// Epub::builder()
    ///     .published_date("2025-10-26")
    ///     // Passing a concrete date instance
    ///     .published_date(Date::new(2025, 10, 26))
    ///     .published_date(DateTime::now())
    ///     // Integration tip: You can use datetime libraries of your choice
    ///     // (e.g., time, chrono, jiff) by converting them to a string.
    ///     .published_date(iso_datetime.to_string())
    ///     // Explicit Builder
    ///     .published_date(DetachedEpubMetaEntry::date("2025-10-26"));
    /// ```
    pub fn published_date(mut self, date: impl Into<DetachedEpubMetaEntry<marker::Date>>) -> Self {
        self.epub.metadata_mut().retain(|entry| {
            if entry.property().as_str() != dc::DATE {
                return true;
            }
            // Check if opf:event attribute exists
            match entry.attributes().by_name(opf::OPF_EVENT) {
                Some(event) if event.value() == opf::PUBLICATION => false,
                Some(_) => true,
                // Plain `dc:date` element (publish date)
                None => false,
            }
        });
        self.epub.metadata_mut().insert(
            0,
            // Force the property and kind because `marker::Date` can also imply `dcterms:modified`.
            date.into()
                .force_property(dc::DATE)
                .force_kind(EpubMetaEntryKind::DublinCore {}),
        );
        self
    }

    /// Sets the modification date (`dcterms:modified`).
    ///
    /// The given date is not validated.
    /// However, a date conforming to
    /// [**ISO 8601-1**](https://www.iso.org/iso-8601-date-and-time-format.html)
    /// is strongly recommended.
    ///
    /// To ensure there is exactly **one** dcterms modification date,
    /// this method finds and removes any existing `dcterms:modified` meta elements.
    ///
    /// # Note
    /// - This is primarily an EPUB 3 feature.
    ///   When [writing](Self::write) an EPUB 2 ebook, this field is ignored.
    ///   However, if a legacy `dc:date` with `opf:event="modification"` exists,
    ///   its value is updated to match the new date.
    ///
    /// # Examples
    /// - Setting the modification date:
    /// ```
    /// use rbook::Epub;
    /// use rbook::ebook::metadata::datetime::{Date, DateTime, Time};
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// # let iso_datetime = "";
    /// Epub::builder()
    ///     .modified_date("2025-12-25T10:32:05Z")
    ///     // Passing a concrete date time instance
    ///     .modified_date(Date::new(2025, 12, 25).at(Time::utc(10, 32, 05)))
    ///     .modified_date(DateTime::now())
    ///     // Integration tip: You can use datetime libraries of your choice
    ///     // (e.g., time, chrono, jiff) by converting them to a string.
    ///     .modified_date(iso_datetime.to_string())
    ///     // Explicit Builder
    ///     .modified_date(DetachedEpubMetaEntry::date("2025-12-25T10:32:05Z"));
    /// ```
    pub fn modified_date(mut self, date: impl Into<DetachedEpubMetaEntry<marker::Date>>) -> Self {
        let date = date.into();
        let _ = self.epub.metadata_mut().remove_by_property(dc::MODIFIED);

        // Sync legacy, if any
        for mut dc_date in self.epub.metadata_mut().by_property_mut(dc::DATE) {
            let opf_event = dc_date.as_view().attributes().get_value(opf::OPF_EVENT);

            if let Some(opf::MODIFICATION) = opf_event {
                dc_date.set_value(date.as_view().value());
            }
        }

        self.meta(
            // Force the property and kind because `marker::Date` can also imply `dc:date`.
            date.force_property(dc::MODIFIED)
                .force_kind(EpubMetaEntryKind::Meta {
                    version: EpubVersion::EPUB3,
                }),
        )
    }

    /// Sets the modification date (`dcterms:modified`) to the current [`DateTime`].
    ///
    /// When modifying an existing EPUB, it is recommended to call this
    /// method before [writing](Self::write).
    ///
    /// If an [`Epub`] is newly created via [`Epub::new`] or [`Epub::builder`],
    /// calling this method is optional, as the modification date is
    /// generated automatically upon writing.
    /// See the [`epub`](crate::epub) module doc for generation details.
    ///
    /// This method serves as a convenient helper, so instead of:
    /// ```
    /// # use rbook::Epub;
    /// use rbook::ebook::metadata::datetime::DateTime;
    ///
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// # Epub::new().edit()
    /// .modified_date(DateTime::now());
    /// # Ok(())
    /// # }
    /// ```
    /// This method can be called instead:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// # Epub::new().edit()
    /// .modified_now();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # WebAssembly
    /// On `wasm32-unknown-unknown`, this method has no effect.
    /// The date can be explicitly given using [`Self::modified_date`].
    pub fn modified_now(self) -> Self {
        match DateTime::try_now() {
            Some(now) => self.modified_date(now),
            _ => self,
        }
    }

    /// Appends one or more titles (`dc:title`) via the [`Many`] trait.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::title`] to explicitly use a builder for greater control.
    ///
    /// # Examples
    /// - Appending titles:
    /// ```
    /// # use rbook::Epub;
    /// use rbook::ebook::metadata::TitleKind;
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// let epub = Epub::builder()
    ///     // Single entry
    ///     .title("Title A")
    ///     // Batch entries
    ///     .title(["B", "C"])
    ///     // Explicit builder
    ///     .title(
    ///         DetachedEpubMetaEntry::title("Example EPUB")
    ///             .alternate_script("ja", "サンプルEPUB")
    ///             .kind(TitleKind::Short)
    ///     )
    ///     .build();
    /// ```
    pub fn title(self, input: impl Many<DetachedEpubMetaEntry<marker::Title>>) -> Self {
        self.meta(Batch(input.iter_many()))
    }

    /// Appends one or more creators (`dc:creator`) with
    /// the **author** role via the [`Many`] trait.
    ///
    /// If any of the given entries already contain roles,
    /// they are treated as subsequent (lower-precedence).
    /// The author (`aut`) role is inserted only if it is not already present.
    ///
    /// If an entry has a legacy EPUB 2 `opf:role` attribute present,
    /// it is set to `aut`.
    ///
    /// # See Also
    /// - [`Self::creator`] to append a creator without the author (`aut`) role.
    ///
    /// # Examples
    /// - Appending authors:
    /// ```
    /// # use rbook::Epub;
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// Epub::builder()
    ///     // Single entry
    ///     .author("John Doe")
    ///     // Batch entries
    ///     .author(["Jane Doe", "Joe Shmoe"])
    ///     // Explicit builder
    ///     .author(
    ///         DetachedEpubMetaEntry::creator("Hanako Yamada")
    ///             // Subsequent roles are treated as secondary
    ///             .role("ill")
    ///             .role("trl")
    ///             .file_as("Yamada, Hanako")
    ///             .alternate_script("ja", "山田花子"),
    ///     )
    ///     // Manually specifying a creator with the author role
    ///     .creator(DetachedEpubMetaEntry::creator("Taro Yamada").role("aut"));
    /// ```
    pub fn author(self, input: impl Many<DetachedEpubMetaEntry<marker::Contributor>>) -> Self {
        self.creator(Batch(input.iter_many().map(|mut author| {
            // NOTE: If the current version is EPUB 2, refinements are downgraded on write.
            let mut entry = author.as_mut();

            // If an EPUB 2 attribute is present, update it to author.
            let replace_role_attribute = entry
                .attributes_mut()
                .get_value(opf::OPF_ROLE)
                .is_some_and(|role| role != marc::AUTHOR);
            // Check if the author role refinement is not present
            let needs_author_role_refinement = entry
                .as_view()
                .refinements()
                .by_property(opf::ROLE)
                .all(|role| role.value() != marc::AUTHOR);

            if needs_author_role_refinement {
                // Ensures the author role has the highest precedence
                entry.refinements_mut().insert(
                    0,
                    DetachedEpubMetaEntry::meta(opf::ROLE)
                        .attribute((opf::SCHEME, marc::RELATORS))
                        .value(marc::AUTHOR),
                );
            }
            // Update legacy role attribute
            if replace_role_attribute {
                // This avoids cases where an EPUB 3 reader sees an author,
                // while an EPUB 2 reader sees an illustrator
                entry.attributes_mut().insert((opf::OPF_ROLE, marc::AUTHOR));
            }

            author
        })))
    }

    /// Appends one or more creators (`dc:creator`) via the [`Many`] trait.
    ///
    /// # See Also
    /// - [`Self::author`] to append a `creator` with the **author** role.
    /// - [`DetachedEpubMetaEntry::creator`] to explicitly use a builder for greater control.
    ///
    /// # Examples
    /// - Appending creators:
    /// ```
    /// # use rbook::Epub;
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// Epub::builder()
    ///     // Single entry
    ///     .creator("John Doe")
    ///     // Batch entries
    ///     .creator(["Jane Doe", "Joe Shmoe"])
    ///     // Explicit builder
    ///     .creator(
    ///         DetachedEpubMetaEntry::creator("Hanako Yamada")
    ///             .file_as("Yamada, Hanako")
    ///             .alternate_script("ja", "山田花子")
    ///             // Explicitly specifying the role as `author` and `illustrator`
    ///             .role("aut")
    ///             .role("ill"),
    ///     );
    /// ```
    pub fn creator(self, input: impl Many<DetachedEpubMetaEntry<marker::Contributor>>) -> Self {
        // Ensure the property is `dc:creator`
        self.insert_checked_meta(dc::CREATOR, input)
    }

    /// Appends one or more contributors (`dc:contributor`) via the [`Many`] trait.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::contributor`] to explicitly use a builder for greater control.
    ///
    /// # Examples
    /// - Appending contributors:
    /// ```
    /// # use rbook::Epub;
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// Epub::builder()
    ///     // Single entry
    ///     .contributor("John Doe")
    ///     // Batch entries
    ///     .contributor(["Jane Doe", "Joe Shmoe"])
    ///     // Explicit builder
    ///     .contributor(
    ///         DetachedEpubMetaEntry::contributor("Hanako Yamada")
    ///             .id("contributor4")
    ///             .file_as("Yamada, Hanako")
    ///             .alternate_script("ja", "山田花子")
    ///             // Specifying the role as `editor`
    ///             .role("edt"),
    ///     );
    /// ```
    pub fn contributor(self, input: impl Many<DetachedEpubMetaEntry<marker::Contributor>>) -> Self {
        // Ensure the property is `dc:contributor`
        self.insert_checked_meta(dc::CONTRIBUTOR, input)
    }

    /// Appends one or more publishers (`dc:publisher`) via the [`Many`] trait.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::publisher`] to explicitly use a builder for greater control.
    ///
    /// # Examples
    /// - Appending publishers:
    /// ```
    /// # use rbook::Epub;
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// Epub::builder()
    ///     // Single entry
    ///     .publisher("Publisher A")
    ///     // Batch entries
    ///     .publisher(["Publisher B", "Publisher C"])
    ///     // Explicit builder
    ///     .publisher(
    ///         DetachedEpubMetaEntry::publisher("Publisher D")
    ///             .file_as("D, Publisher")
    ///             .alternate_script("ja", "D出版社"),
    ///     );
    /// ```
    pub fn publisher(self, input: impl Many<DetachedEpubMetaEntry<marker::Contributor>>) -> Self {
        // Ensure the property is `dc:publisher`
        self.insert_checked_meta(dc::PUBLISHER, input)
    }

    /// Appends one or more tags (`dc:subject`) via the [`Many`] trait.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::tag`] to explicitly use a builder for greater control.
    ///
    /// # Examples
    /// - Appending publishers:
    /// ```
    /// # use rbook::Epub;
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// Epub::builder()
    ///     // Single entry
    ///     .tag("Adventure")
    ///     // Batch entries
    ///     .tag(["Fantasy", "Science fiction"])
    ///     // Explicit builder for standardized tags
    ///     .tag(
    ///         DetachedEpubMetaEntry::tag("FICTION / Occult & Supernatural")
    ///             .scheme("BISAC", "FIC024000"),
    ///     );
    /// ```
    pub fn tag(self, input: impl Many<DetachedEpubMetaEntry<marker::Tag>>) -> Self {
        self.meta(Batch(input.iter_many()))
    }

    /// Appends one or more descriptions (`dc:description`) via the [`Many`] trait.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::description`] to explicitly use a builder for greater control.
    pub fn description(self, input: impl Many<DetachedEpubMetaEntry<marker::Description>>) -> Self {
        self.meta(Batch(input.iter_many()))
    }

    /// Appends one or more languages (`dc:language`) via the [`Many`] trait.
    ///
    /// The given language code is not validated and ***should*** be a valid
    /// [BCP 47](https://tools.ietf.org/html/bcp47) tag (e.g. `en`, `ja`, `fr-CA`).
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::language`] to explicitly use a builder for greater control.
    ///
    /// # Examples
    /// - Setting the language of a newly created [`Epub`]:
    /// ```
    /// # use rbook::Epub;
    /// let epub = Epub::builder()
    ///     .language("en")
    ///     .build();
    /// ```
    pub fn language(self, input: impl Many<DetachedEpubMetaEntry<marker::Language>>) -> Self {
        self.meta(Batch(input.iter_many()))
    }

    /// Appends one or more disclaimers/copyright/licenses (`dc:rights`)
    /// via the [`Many`] trait.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::rights`] to explicitly use a builder for greater control.
    ///
    /// # Examples
    /// - Setting the rights of a newly created [`Epub`]:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let epub = Epub::builder()
    ///     // Single entry
    ///     .rights(
    ///         "This ebook dedicate their contributions to the worldwide public domain via the terms in the \
    ///         [CC0 1.0 Universal Public Domain Dedication](https://creativecommons.org/publicdomain/zero/1.0/).",
    ///     )
    ///     // Batch entries
    ///     .rights([
    ///         "GNU General Public License v3.0",
    ///         "CC Creative Commons BY-SA 4.0",
    ///     ])
    ///     // Explicit builder
    ///     .meta(
    ///         DetachedEpubMetaEntry::dublin_core("dc:rights")
    ///             .value("Apache License 2.0")
    ///             .refinement(
    ///                 DetachedEpubMetaEntry::link("dcterms:rights")
    ///                     .href("https://www.apache.org/licenses/LICENSE-2.0")
    ///             ),
    ///     )
    ///     .build();
    /// ```
    pub fn rights(self, input: impl Many<DetachedEpubMetaEntry<marker::Rights>>) -> Self {
        self.meta(Batch(input.iter_many()))
    }

    /// Sets the generator, replacing any previous.
    ///
    /// A generator indicates the software used to create an [`Epub`].
    /// It can be removed by passing [`None`] or overridden to another preferred name.
    ///
    /// # Default Generator
    /// If an [`Epub`] is created using [`Epub::new`] or [`Epub::builder`],
    /// then `rbook` is the default generator, aiding in debugging if the produced file
    /// has issues.
    ///
    /// # Examples
    /// - Setting the generator of an [`Epub`]:
    /// ```
    /// # use rbook::Epub;
    /// let epub = Epub::builder()
    ///     .generator("My App") // Sets generator to `My App`
    ///     .generator(None);    // Removes the generator entry
    /// ```
    pub fn generator(mut self, generator: impl IntoOption<String>) -> Self {
        // Remove the previous generator, if any
        let _ = self.epub.metadata_mut().remove_by_property(opf::GENERATOR);

        match generator.into_option() {
            Some(generator) => {
                self.meta(DetachedEpubMetaEntry::meta_name(opf::GENERATOR).value(generator))
            }
            None => self,
        }
    }

    /// Appends one or more metadata entries via the [`Many`] trait.
    ///
    /// The [`value`](DetachedEpubMetaEntry::value) of metadata entries is allowed
    /// to be *escaped* or *unescaped* XML.
    ///
    /// # See Also
    /// A builder may be passed as an argument for greater control:
    /// - [`DetachedEpubMetaEntry::dublin_core`]
    /// - [`DetachedEpubMetaEntry::link`]
    /// - [`DetachedEpubMetaEntry::meta`]
    ///
    /// # Examples
    /// - Appending metadata:
    /// ```
    /// # use rbook::Epub;
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// Epub::builder()
    ///     // Single entry (name/property, value/content)
    ///     .meta(("dc:rights", "Apache License 2.0"))
    ///     // Explicit builder
    ///     .meta(
    ///         DetachedEpubMetaEntry::dublin_core("dc:rights")
    ///             .value("Apache License 2.0")
    ///             .refinement(
    ///                 DetachedEpubMetaEntry::link("dcterms:rights")
    ///                     .href("https://www.apache.org/licenses/LICENSE-2.0")
    ///             ),
    ///     );
    /// ```
    pub fn meta(mut self, detached: impl Many<DetachedEpubMetaEntry>) -> Self {
        self.epub.metadata_mut().push(detached);
        self
    }

    /// Removes **all** non-refining metadata entries by the given
    /// [`property`](crate::epub::metadata::EpubMetaEntry::property).
    ///
    /// # Note
    /// This method is equivalent to calling
    /// [`EpubMetadataMut::remove_by_property`](crate::epub::metadata::EpubMetadataMut::remove_by_property)
    /// *without* returning any of the removed entries.
    ///
    /// # Examples
    /// - Replacing the creator of an [`Epub`]:
    /// ```no_run
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// Epub::open("tests/ebooks/example_epub")?
    ///     .edit()
    ///     // Clears all creators (even if there are several)
    ///     .clear_meta("dc:creator")
    ///     // Add Jane Doe as the sole creator/author
    ///     .author("Jane Doe")
    ///     // Setting the modified date to now
    ///     .modified_now()
    ///     .write()
    ///     .save("new.epub");
    /// # Ok(())
    /// # }
    /// ```
    pub fn clear_meta(mut self, property: impl AsRef<str>) -> Self {
        let _ = self
            .epub
            .metadata_mut()
            .remove_by_property(property.as_ref());
        self
    }

    //////////////////////////////////
    // MANIFEST
    //////////////////////////////////

    /// Inserts one or more resources into the manifest via the [`Many`] trait.
    ///
    /// # Path Resolution
    /// Resources are stored relative to the
    /// [**directory**](crate::epub::package::EpubPackage::directory)
    /// where the package file is stored (e.g. `OEBPS`, `EPUB`).
    /// It is **not** recommended to prepend the package directory to paths.
    ///
    /// Relative paths are resolved automatically.
    /// For example, if the package directory is `/OEBPS`:
    /// - `styles.css` → `/OEBPS/styles.css`
    /// - `images/1.jpg` → `/OEBPS/images/1.jpg` (Nested)
    /// - `../frames.smil` → `/frames.smil` (Parent)
    /// - `OEBPS/images/1.jpg` → `/OEBPS/OEBPS/images/1.jpg` (Duplicate prefix)
    ///
    /// Relative path resolution can be bypassed by providing an absolute path
    /// (prefixed with `/`), which is used as-is:
    /// - `/META-INF/foo.xml` → `/META-INF/foo.xml` (Absolute path)
    ///
    /// # Auto-Generated ID and Media Type Detection
    /// For each resource given, the `media type` is automatically inferred
    /// from the file extension and a unique `id` is generated from the href.
    ///
    /// Extensions that support media type detection:
    ///
    /// | **Image** Extensions | Media Type      |
    /// |----------------------|-----------------|
    /// | `jpg`/`jpeg`         | `image/jpeg`    |
    /// | `png`                | `image/png`     |
    /// | `svg`                | `image/svg+xml` |
    /// | `gif`                | `image/gif`     |
    /// | `webp`               | `image/webp`    |
    ///
    /// | **Text** Extensions | Media Type                 |
    /// |---------------------|----------------------------|
    /// | `xhtml`             | `application/xhtml+xml`    |
    /// | `html`/`htm`        | `text/html`                |
    /// | `css`               | `text/css`                 |
    /// | `js`                | `text/javascript`          |
    /// | `smil`              | `application/smil+xml`     |
    /// | `ncx`               | `application/x-dtbncx+xml` |
    /// | `xml`               | `application/xml`          |
    ///
    /// | **Font** Extensions | Media Type   |
    /// |---------------------|--------------|
    /// | `ttf`               | `font/ttf`   |
    /// | `otf`               | `font/otf`   |
    /// | `woff`              | `font/woff`  |
    /// | `woff2`             | `font/woff2` |
    ///
    /// | **Audio** Extensions | Media Type   |
    /// |----------------------|--------------|
    /// | `mp3`                | `audio/mpeg` |
    /// | `m4a`                | `audio/mp4`  |
    /// | `aac`                | `audio/aac`  |
    ///
    /// | **Video** Extensions  | Media Type   |
    /// |-----------------------|--------------|
    /// |`mp4`/`m4v`            | `video/mp4`  |
    /// |`webm`                 | `video/webm` |
    ///
    /// If the extension is unrecognized, `application/octet-stream` is used.
    ///
    /// # See Also
    /// - [`Self::package_location`] to change the package directory/file location.
    /// - [`Self::container_resource`] to insert a resource without adding it to the manifest.
    /// - [`ResourceContent`] for details on providing data from memory (bytes/strings)
    ///   or the OS file system (paths).
    ///
    /// # Examples
    /// - Inserting resources:
    /// ```
    /// # const IMAGE_WEBP_BYTES: &[u8] = &[];
    /// # const IMAGE_PNG_BYTES: &[u8] = &[];
    /// # const IMAGE_JPG_BYTES: &[u8] = &[];
    /// # const CSS_BYTES: &[u8] = &[];
    /// # use std::path::PathBuf;
    /// # use rbook::Epub;
    /// # use rbook::epub::manifest::DetachedEpubManifestEntry;
    ///
    /// Epub::builder()
    ///     // Single entry (href, bytes)
    ///     .resource(("images/pic0.webp", IMAGE_WEBP_BYTES))
    ///     // Batch entries
    ///     .resource([
    ///         ("images/pic2.png", IMAGE_PNG_BYTES),
    ///         ("images/pic3.jpg", IMAGE_JPG_BYTES),
    ///     ])
    ///     // Inserting resources referencing a file stored on the OS file system
    ///     .resource([
    ///         ("c1_overlay.smil", PathBuf::from("local/overlays/c1.smil")),
    ///         ("font.woff", PathBuf::from("local/fonts/main.woff")),
    ///     ])
    ///     // Explicit builder
    ///     .resource(
    ///         DetachedEpubManifestEntry::new("css-9-id")
    ///             .media_type("text/css")
    ///             .href("styles/supplementary.css")
    ///             .content(CSS_BYTES),
    ///     );
    /// ```
    pub fn resource(mut self, resource: impl Many<DetachedEpubManifestEntry>) -> Self {
        self.epub.manifest_mut().push(resource);
        self
    }

    /// Sets the cover image.
    ///
    /// If a cover image entry already exists, it is unmarked as the cover image
    /// and retained within the manifest. If removing the entry is preferred, see
    /// [`EpubManifestMut::retain`](crate::epub::manifest::EpubManifestMut::retain)
    /// to remove the entry using a predicate.
    ///
    /// If present, the legacy EPUB 2 metadata cover entry is updated to reference
    /// the new cover image entry.
    ///
    /// # See Also
    /// - [`EpubEditor::resource`] for path (href) resolution and inferred media
    ///   type details of the inserted cover image resource.
    /// - [`EpubManifestMut::cover_image_mut`](crate::epub::manifest::EpubManifestMut::cover_image_mut)
    ///   to modify an existing cover image entry directly.
    ///
    /// # Examples
    /// - Setting a cover image from a file on disk:
    /// ```
    /// # use std::path::PathBuf;
    /// # use rbook::Epub;
    /// Epub::builder()
    ///     .cover_image((
    ///         // The location where the resource will be stored within the EPUB.
    ///         "cover.png",
    ///         // The location of the source file on the OS file system.
    ///         PathBuf::from("local/final/cover9.png"),
    ///     ));
    /// ```
    /// - Alternatively, if the image is already in memory, its raw bytes can be passed:
    /// ```
    /// # use rbook::Epub;
    /// # let image_data = Vec::new();
    /// let cover_bytes: Vec<u8> = image_data;
    ///
    /// Epub::builder()
    ///     .cover_image(("cover.png", cover_bytes));
    /// ```
    pub fn cover_image(mut self, resource: impl Into<DetachedEpubManifestEntry>) -> Self {
        // Ensure the given resource has the cover-image property
        let resource = resource.into().property(opf::COVER_IMAGE);

        // Remove any previous cover-image property
        self.epub.manifest_mut().for_each_mut(|entry| {
            entry.properties_mut().remove(opf::COVER_IMAGE);
        });

        // Update legacy EPUB 2 cover meta attribute (There should be one cover entry)
        if let Some(mut cover_meta) = self.epub.metadata_mut().by_property_mut(opf::COVER).next() {
            cover_meta.set_value(resource.as_view().id());
        }

        self.resource(resource)
    }

    //////////////////////////////////
    // SPINE
    //////////////////////////////////

    /// Sets the [`PageDirection`] hint of an [`Epub`], indicating how readable content flows.
    ///
    /// This method is equivalent to calling
    /// [`EpubSpineMut::set_page_direction`](crate::epub::spine::EpubSpineMut::set_page_direction)
    /// without returning the previous direction.
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](Self::write) an EPUB 2 ebook, this field is ignored.
    ///
    /// Default: [`PageDirection::Default`]
    pub fn page_direction(mut self, direction: PageDirection) -> Self {
        self.epub.spine_mut().set_page_direction(direction);
        self
    }

    //////////////////////////////////
    // ToC
    //////////////////////////////////

    /// Sets the title of the main table of contents, replacing any previous title.
    ///
    /// # Examples
    /// - Setting the title of the table of contents:
    /// ```
    /// # const C1_XHTML: &[u8] = &[];
    /// # const C2_XHTML: &[u8] = &[];
    /// # const C3_XHTML: &[u8] = &[];
    /// # use rbook::Epub;
    /// use rbook::epub::EpubChapter;
    ///
    /// Epub::builder()
    ///     // Implicitly add chapters to the table of contents
    ///     .chapter([
    ///         EpubChapter::new("Chapter I").href("c1.xhtml").xhtml(C1_XHTML),
    ///         EpubChapter::new("Chapter II").href("c2.xhtml").xhtml(C2_XHTML),
    ///         EpubChapter::new("Chapter III").href("c3.xhtml").xhtml(C3_XHTML),
    ///     ])
    ///     // Setting the title
    ///     .toc_title("Chapters");
    /// ```
    ///
    /// Default: `Table of Contents`
    pub fn toc_title(mut self, title: impl Into<String>) -> Self {
        self.set_toc_root_label(TocEntryKind::Toc, title.into());
        self
    }

    /// Sets the title of the landmarks navigation, replacing any previous title.
    ///
    /// Landmarks identify structural points of interest within an ebook
    /// (e.g., [Title Page](TocEntryKind::TitlePage), [Glossary](TocEntryKind::Glossary),
    /// [Copyright](TocEntryKind::CopyrightPage)).
    ///
    /// # Note
    /// Setting the landmarks title is an EPUB 3 feature.
    /// When [writing](Epub::write) an EPUB 2 ebook, the given title is ignored.
    ///
    /// # Examples
    /// - Setting the title of the landmarks:
    /// ```
    /// # const INTRO_XHTML: &[u8] = &[];
    /// # const SECTION_1_XHTML: &[u8] = &[];
    /// # const APPENDIX_XHTML: &[u8] = &[];
    /// # use rbook::Epub;
    /// use rbook::ebook::toc::TocEntryKind;
    /// use rbook::epub::EpubChapter;
    ///
    /// Epub::builder()
    ///     // Implicitly add chapters to landmarks by specifying `EpubChapter::kind`
    ///     .chapter([
    ///         EpubChapter::new("Introduction")
    ///             // Add to landmarks as the introduction
    ///             .kind(TocEntryKind::Introduction)
    ///             .href("intro.xhtml")
    ///             .xhtml(INTRO_XHTML),
    ///         // Entry not added to landmarks as the kind is not specified
    ///         EpubChapter::new("Section 1")
    ///             .href("section-1.xhtml")
    ///             .xhtml(SECTION_1_XHTML),
    ///         EpubChapter::new("Appendix")
    ///             // Add to landmarks as the appendix
    ///             .kind(TocEntryKind::Appendix)
    ///             .href("appendix.xhtml")
    ///             .xhtml(APPENDIX_XHTML),
    ///     ])
    ///     // Setting the title
    ///     .landmarks_title("Points of Interest");
    /// ```
    ///
    /// Default: `Landmarks`
    pub fn landmarks_title(mut self, title: impl Into<String>) -> Self {
        self.set_toc_root_label(TocEntryKind::Landmarks, title.into());
        self
    }

    fn set_toc_root_label(&mut self, kind: TocEntryKind, title: String) {
        let mut found = false;

        // Try to find and update existing roots
        for mut root in self.epub.toc_mut() {
            if root.as_view().kind() == kind {
                root.set_label(&title);
                found = true;
            }
        }

        // Mostly likely created using Epub::new or Epub::create, so create a new toc root.
        if !found {
            let version = self.epub.package().version();
            self.epub
                .toc_mut()
                .insert_root(kind, version, DetachedEpubTocEntry::new(title));
        }
    }

    //////////////////////////////////
    // MANIFEST, SPINE, TOC
    //////////////////////////////////

    /// Appends one or more [chapters](EpubChapter) via the [`Many`] trait.
    ///
    /// # See Also
    /// - [`EpubEditor::resource`] to insert resources (e.g., images, audio)
    ///   not part of the canonical reading order.
    ///
    /// # Examples
    /// - Appending chapters:
    /// ```
    /// # const C2_XHTML: &[u8] = &[];
    /// # const C3_XHTML: &[u8] = &[];
    /// # use rbook::Epub;
    /// use rbook::epub::EpubChapter;
    ///
    /// Epub::builder()
    ///     // Single entry
    ///     .chapter(EpubChapter::new("Chapter I").xhtml_body("<b>Hello world!</b>"))
    ///     // Bulk entries
    ///     .chapter([
    ///         EpubChapter::new("Chapter II").xhtml(C2_XHTML),
    ///         EpubChapter::new("Chapter III").xhtml(C3_XHTML),
    ///     ]);
    pub fn chapter(mut self, chapter: impl Many<EpubChapter>) -> Self {
        for chapter in chapter.iter_many() {
            if let Some(toc_entry) = self.dfs_process_chapter(chapter) {
                self.insert_into_toc(TocEntryKind::Toc, toc_entry);
            }
        }
        self
    }

    fn insert_into_toc(&mut self, kind: TocEntryKind, entry: DetachedEpubTocEntry) {
        if let Some(mut toc) = self.epub.toc_mut().by_kind_mut(kind) {
            toc.push(entry);
        } else {
            let version = self.epub.package().version();
            let default_title = match kind {
                TocEntryKind::Landmarks => Self::DEFAULT_LANDMARKS_TITLE,
                _ => Self::DEFAULT_TOC_TITLE,
            };

            self.epub.toc_mut().insert_root(
                kind,
                version,
                DetachedEpubTocEntry::new(default_title).children(entry),
            );
        }
    }

    fn dfs_process_chapter(&mut self, mut chapter: EpubChapter) -> Option<DetachedEpubTocEntry> {
        self.insert_chapter_resource(&mut chapter);
        self.insert_chapter_landmarks(&mut chapter);

        for sub in chapter.sub_chapters {
            if let Some(child_toc_entry) = self.dfs_process_chapter(sub)
                // Build toc hierarchy
                && let Some(parent) = &mut chapter.toc_entry
            {
                parent.as_mut().push(child_toc_entry);
            }
            // NOTE: If the parent has no toc entry, but the child does, it is ignored.
        }
        // Return the built toc entry
        chapter.toc_entry
    }

    /// Appends to landmarks if there is a semantic kind present.
    fn insert_chapter_landmarks(&mut self, chapter: &mut EpubChapter) {
        if let Some(entry) = &mut chapter.toc_entry
            // If there's a semantic kind, add it to the landmarks
            && entry.as_view().kind_raw().is_some()
        {
            let landmarks_entry = entry.clone();

            self.insert_into_toc(TocEntryKind::Landmarks, landmarks_entry);
        }
    }

    fn insert_chapter_resource(&mut self, chapter: &mut EpubChapter) {
        let Some(mut manifest_entry) = chapter.manifest_entry.take() else {
            return;
        };

        // There must be an associated spine entry
        let mut spine_entry = chapter
            .spine_entry
            .take()
            .unwrap_or_else(|| DetachedEpubSpineEntry::new(String::new()));

        self.process_manifest_entry(
            &mut manifest_entry,
            chapter
                .toc_entry
                .as_ref()
                .map(|entry| entry.as_view().label()),
        );

        // Replace the href if it is a placeholder
        if manifest_entry.as_view().href_raw().as_str().is_empty() {
            // Avoid conflicting hrefs
            let href = self
                .epub
                .manifest
                .generate_unique_href(util::str::suffix(".xhtml", manifest_entry.as_view().id()));

            // Sync toc entry
            if let Some(toc_entry) = chapter.toc_entry.as_mut() {
                toc_entry.as_mut().set_href(Some(href.clone()));
            }
            manifest_entry.as_mut().set_href(href);
        }

        // Set media type
        manifest_entry.as_mut().set_media_type(mime::XHTML);
        // Sync spine entry
        spine_entry
            .as_mut()
            .set_idref(manifest_entry.as_view().id());

        self.epub.manifest_mut().push(manifest_entry);
        self.epub.spine_mut().push(spine_entry);
    }

    //////////////////////////////////
    // TERMINAL
    //////////////////////////////////

    /// Returns configuration to write an [`Epub`] to a destination.
    ///
    /// # See Also
    /// - [`Epub::write`] to write to a destination directly from an [`Epub`].
    #[must_use]
    pub fn write(&self) -> EpubWriteOptions<&Epub> {
        self.epub.write()
    }
}

/// A high-level builder to add readable content to an [`Epub`]
/// (e.g., chapters, sections, frontmatter, backmatter).
///
/// An [`EpubChapter`] represents a unique navigable section for end-user reading, such as:
/// - **Narrative Chapters** (e.g., "Chapter 1")
/// - **Frontmatter** (e.g., Title Page, Dedication)
/// - **Backmatter** (e.g., Appendix, Copyright)
/// - **Grouping Headers** (e.g., Containing nested entries)
///
/// # Advanced Use-Cases
/// [`EpubChapter`] is an abstraction over the lower-level [`Epub`] write API:
/// - [`EpubManifestMut`](crate::epub::manifest::EpubManifestMut)
/// - [`EpubSpineMut`](crate::epub::spine::EpubSpineMut)
/// - [`EpubTocMut`](crate::epub::toc::EpubTocMut)
///   ([`EpubTocMut::contents_mut`](crate::epub::toc::EpubTocMut::contents_mut),
///   [`EpubTocMut::landmarks_mut`](crate::epub::toc::EpubTocMut::landmarks_mut))
///
/// For advanced use-cases, the lower-level APIs offer greater flexibility.
///
/// # See Also
/// - [`EpubEditor::chapter`] to add an [`EpubChapter`] to an [`Epub`].
/// - [`EpubChapter::supplementary`] to mark content as non-linear.
///
/// # Example
/// - Creating a chapter hierarchy:
/// ```
/// # use rbook::ebook::toc::TocEntryKind;
/// # use rbook::epub::EpubChapter;
/// EpubChapter::new("Volume 3")                // 1. Specify chapter title
///     .kind(TocEntryKind::Volume)             // 2. Specify semantic kind
///     .href("v3.xhtml")                       // 3. Set the location
///     .xhtml_body("<h1>Volume 3: rbook</h1>") // 4. Set the XHTML content
///     .children(                              // 5. Add subchapters
///         EpubChapter::new("Chapter 1")
///             .href("v3c1.xhtml")
///             .xhtml(r#"<html xmlns="http://www.w3.org/1999/xhtml">...</html>"#)
///             .children(
///                 // Adding a fragment
///                 EpubChapter::new("Chapter 1.1").href("v3c1.xhtml#section-1"),
///             ),
///     );
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct EpubChapter {
    sub_chapters: Vec<Self>,
    spine_entry: Option<DetachedEpubSpineEntry>,
    manifest_entry: Option<DetachedEpubManifestEntry>,
    toc_entry: Option<DetachedEpubTocEntry>,
}

impl EpubChapter {
    /// Creates a new [`EpubChapter`] with the given [`title`](Self::title).
    ///
    /// # See Also
    /// - [`Self::unlisted`] to create unlisted chapters.
    ///
    /// # Examples
    /// - Inserting chapters:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::EpubChapter;
    /// # const C1_XHTML: &[u8] = &[];
    /// # const C2_XHTML: &[u8] = &[];
    /// let epub = Epub::builder()
    ///     .chapter([
    ///         EpubChapter::new("Chapter 1").xhtml(C1_XHTML),
    ///         EpubChapter::new("Chapter 2").href("c2.xhtml").xhtml(C2_XHTML),
    ///         EpubChapter::new("Chapter 2.1").href("c2.xhtml#section-1"),
    ///     ])
    ///     .build();
    ///
    /// // Checking the main table of contents:
    /// let contents = epub.toc().contents().unwrap();
    /// assert_eq!(3, contents.len());
    /// assert_eq!("Chapter 1", contents.get(0).unwrap().label());
    /// assert_eq!("Chapter 2", contents.get(1).unwrap().label());
    /// assert_eq!("Chapter 2.1", contents.get(2).unwrap().label());
    ///
    /// // Checking the manifest/spine:
    /// assert_eq!(2, epub.spine().len());
    /// assert_eq!(2, epub.manifest().len());
    /// ```
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            sub_chapters: Vec::new(),
            spine_entry: None,
            manifest_entry: None,
            toc_entry: Some(DetachedEpubTocEntry::new(title)),
        }
    }

    /// Creates a new [`EpubChapter`] that will ***not*** be added to the main table of contents
    /// ([`EpubTocMut::contents_mut`](crate::epub::toc::EpubTocMut::contents_mut)).
    ///
    /// The given [`href`](Self::href) sets the location where the unlisted resource
    /// will be stored within an EPUB.
    ///
    /// # Note
    /// An instance created through this method can still be added to the landmarks
    /// by specifying the [`Self::kind`] field.
    ///
    /// # Examples
    /// - Inserting unlisted chapters:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::EpubChapter;
    /// # const C1_XHTML: &[u8] = &[];
    /// # const C2_XHTML: &[u8] = &[];
    /// # const C3_XHTML: &[u8] = &[];
    /// let epub = Epub::builder()
    ///     .chapter([
    ///         EpubChapter::new("Chapter 1").href("c1.xhtml").xhtml(C1_XHTML),
    ///         // Inserting content unlisted from the main ToC contents:
    ///         EpubChapter::unlisted("c2.xhtml").xhtml(C2_XHTML),
    ///         EpubChapter::unlisted("c3.xhtml").xhtml(C3_XHTML),
    ///     ])
    ///     .build();
    ///
    /// let contents = epub.toc().contents().unwrap();
    /// // Because the other inserted chapters are unlisted,
    /// // they do not appear in the main toc contents.
    /// assert_eq!(1, contents.len());
    /// assert_eq!("Chapter 1", contents.get(0).unwrap().label());
    ///
    /// // Unlisted chapters are part of the canonical
    /// // reading order and are added to the manifest/spine
    /// assert_eq!(3, epub.manifest().len());
    /// assert_eq!(3, epub.spine().len());
    /// ```
    pub fn unlisted(href: impl Into<String>) -> Self {
        Self {
            sub_chapters: Vec::new(),
            spine_entry: None,
            manifest_entry: Some(
                // Manifest entry `id` temporarily empty
                DetachedEpubManifestEntry::new(String::new()),
            ),
            toc_entry: None,
        }
        .href(href)
    }

    /// Override the contained spine entry for advanced use-cases
    /// (e.g. refinements, properties, attributes).
    /// It is strongly recommended to call this method before any other method.
    ///
    /// The given entry is retained as-is at the time of this method call
    /// and will be staged for inserted into
    /// [`EpubSpineMut`](crate::epub::spine::EpubSpineMut).
    ///
    /// The `id` of the referenced manifest entry ([`DetachedEpubSpineEntry::idref`])
    /// will be **overridden** when inserted into [`EpubEditor`].
    ///
    /// # Examples
    /// - Providing a spine entry:
    /// ```
    /// # use rbook::epub::spine::DetachedEpubSpineEntry;
    /// # use rbook::epub::EpubChapter;
    /// EpubChapter::new("Answer Sheet")
    ///     .with_spine_entry(
    ///         // The given idref will be overridden, so passing an empty string is valid.
    ///         DetachedEpubSpineEntry::new("")
    ///             // Setting the readable content as supplementary
    ///             .linear(false)
    ///             // Adding a property
    ///             .property("page-spread-left"),
    ///     );
    /// ```
    pub fn with_spine_entry(mut self, entry: DetachedEpubSpineEntry) -> Self {
        self.spine_entry = Some(entry);
        self
    }

    /// Override the contained manifest entry for advanced use-cases
    /// (e.g. explicit entry id, refinements, media overlays, properties).
    /// It is strongly recommended to call this method before any other method
    /// (e.g. [`Self::href`], [`Self::xhtml`]).
    ///
    /// The given `entry` is retained as-is at the time of this method call and
    /// will be staged for insertion into
    /// [`EpubManifestMut`](crate::epub::manifest::EpubManifestMut).
    ///
    /// The `media type` of the manifest entry ([`DetachedEpubManifestEntry::media_type`])
    /// will be **overridden** to `application/xhtml+xml` when inserted into [`EpubEditor`].
    ///
    /// # Examples
    /// - Providing a manifest entry:
    /// ```
    /// # use rbook::epub::manifest::DetachedEpubManifestEntry;
    /// # use rbook::epub::EpubChapter;
    /// EpubChapter::new("Chapter 1")
    ///     .with_manifest_entry(
    ///         // Creating a manifest entry with an explicit id `c1`
    ///         DetachedEpubManifestEntry::new("c1")
    ///             // Referencing an overlay by manifest resource id
    ///             .media_overlay("c1_overlay")
    ///             // Adding a duration refinement
    ///             .refinement(("media:duration", "0:32:29"))
    ///     );
    /// ```
    pub fn with_manifest_entry(mut self, entry: DetachedEpubManifestEntry) -> Self {
        self.manifest_entry = Some(entry);
        self
    }

    /// Override the contained toc entry for advanced use-cases.
    /// It is strongly recommended to call this method before any other method
    /// (e.g. [`Self::href`], [`Self::kind`], [`Self::xhtml`]).
    ///
    /// The given `entry` is retained as-is at the time of this method call and
    /// will be staged for insertion into
    /// [`EpubTocMut::contents_mut`](crate::epub::toc::EpubTocMut::contents_mut).
    ///
    /// # Examples
    /// - Providing a toc entry:
    /// ```
    /// # use rbook::ebook::toc::TocEntryKind;
    /// # use rbook::epub::toc::DetachedEpubTocEntry;
    /// # use rbook::epub::EpubChapter;
    /// // Chapter title initially set to an empty string as it will be overridden.
    /// EpubChapter::new("")
    ///     .with_toc_entry(
    ///         // Creating a toc entry with a title
    ///         DetachedEpubTocEntry::new("Chapter 1")
    ///             // Unlike EpubChapter::kind,
    ///             // this doesn't implicitly create a landmarks entry
    ///             .kind(TocEntryKind::Chapter)
    ///     );
    /// ```
    pub fn with_toc_entry(mut self, entry: DetachedEpubTocEntry) -> Self {
        self.toc_entry = Some(entry);
        self
    }

    /// Sets the title of a chapter.
    ///
    /// The title is stored as plain text (e.g. `"1 < 2 & 3"`)
    /// and is XML-escaped automatically during [writing](EpubEditor::write).
    ///
    /// # Note
    /// The title is initially set by [`Self::new`].
    pub fn title(mut self, title: impl Into<String>) -> Self {
        if let Some(toc) = &mut self.toc_entry {
            toc.as_mut().set_label(title);
        }
        self
    }

    /// Sets the location of a chapter relative to the package directory of an [`Epub`].
    ///
    /// # Auto-generated HREF
    /// If no href is provided, it is generated from [`Self::title`] via slugging:
    /// - ASCII alphanumeric characters are retained and decapitalized.
    /// - All other characters are replaced with `-`.
    /// - For example: `Chapter #1: Intro?` → `chapter-1-intro.xhtml`
    ///
    /// If two chapters produce the same slug, a numeric suffix is appended:
    ///
    /// `name.xhtml` → `name1.xhtml` → `name2.xhtml`, etc.
    ///
    /// **Auto-generated hrefs are only recommended when explicit
    /// href references are not required.**
    ///
    /// # Manual HREF
    /// For maximum compatibility with reading systems,
    /// it is recommended to only use alphanumeric characters,
    /// dashes (`-`), and underscores (`_`) in directory and file names.
    ///
    /// - **Malformed**: `My+chapter & #1.xhtml` (Invalid; Not percent-encoded)
    /// - Not recommended: `my%20chapter%20no1.xhtml` (Valid; percent-encoded)
    /// - Recommended: `my_chapter_no1.xhtml` (Valid)
    ///
    /// # Percent Encoding
    /// The given `href` is expected to already be percent encoded.
    /// This method does not check href validity.
    ///
    /// # See Also
    /// - [`EpubEditor::resource`] for path details.
    ///   The same path resolution rules apply to this method.
    ///
    /// # Examples
    /// - Creating a chapter with/without an href:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::EpubChapter;
    /// # const CHAPTER_1_XHTML: &[u8] = &[];
    /// # const CHAPTER_2_XHTML: &[u8] = &[];
    /// Epub::builder()
    ///     .chapter([
    ///         EpubChapter::new("Chapter 1")
    ///             .href("c1.xhtml")
    ///             .xhtml(CHAPTER_1_XHTML),
    ///         // No href defined:
    ///         EpubChapter::new("Chapter 2")
    ///             .xhtml(CHAPTER_2_XHTML),
    ///     ]);
    /// ```
    pub fn href(mut self, href: impl Into<String>) -> Self {
        // Update Manifest and ToC href
        let href = href.into();

        if let Some(entry) = &mut self.manifest_entry {
            entry.as_mut().set_href(&href);
        }
        if let Some(entry) = &mut self.toc_entry {
            entry.as_mut().set_href(href);
        }
        self
    }

    /// Sets the semantic kind (e.g., `titlepage`, `cover`, `chapter`, `epilogue`).
    ///
    /// Setting this field adds the chapter to
    /// [`EpubTocMut::landmarks_mut`](crate::epub::toc::EpubTocMut::landmarks_mut).
    pub fn kind(mut self, kind: impl Into<TocEntryKind<'static>>) -> Self {
        if let Some(toc) = &mut self.toc_entry {
            toc.as_mut().set_kind(kind.into());
        }
        self
    }

    /// Set whether the content is supplementary; not part of the main reading order.
    ///
    /// Supplementary content are best used for content intended to be accessed
    /// via hyperlinks (e.g., answer keys) rather than sequential navigation.
    ///
    /// This will create a [**non-linear**](crate::epub::spine::EpubSpineEntry::is_linear)
    /// spine entry, if the given argument `is_supplementary` is `true`.
    /// Otherwise, a **linear** entry will be created.
    ///
    /// # Examples
    /// - Setting content as supplementary:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::EpubChapter;
    /// # const S1_XHTML: &[u8] = &[];
    /// # const S1_ANSWERS_XHTML: &[u8] = &[];
    /// # const S2_XHTML: &[u8] = &[];
    /// # const S3_XHTML: &[u8] = &[];
    /// let epub = Epub::builder()
    ///     .chapter([
    ///         EpubChapter::new("Section 1").xhtml(S1_XHTML),
    ///         // Make content as supplementary; not part of the main reading order.
    ///         // Reading systems may naturally skip over the entry.
    ///         EpubChapter::new("Answer Sheet").supplementary(true).xhtml(S1_ANSWERS_XHTML),
    ///         EpubChapter::new("Section 2").xhtml(S2_XHTML),
    ///         EpubChapter::new("Section 3").xhtml(S3_XHTML),
    ///     ])
    ///     .build();
    ///
    /// let spine = epub.spine();
    /// // The answer sheet is marked as supplementary; non-linear
    /// assert!(!spine.get(1).unwrap().is_linear());
    /// // All other entries are part of the main reading order (linear)
    /// assert!(spine.get(0).unwrap().is_linear());
    /// assert!(spine.get(2).unwrap().is_linear());
    /// assert!(spine.get(3).unwrap().is_linear());
    /// ```
    pub fn supplementary(mut self, is_supplementary: bool) -> Self {
        let spine_entry = self.spine_entry.get_or_insert_with(|| {
            // The `idref` is temporarily set to an empty string
            DetachedEpubSpineEntry::new(String::new())
        });
        spine_entry.as_mut().set_linear(!is_supplementary);
        self
    }

    /// Sets the XHTML content as-is for end-user reading.
    ///
    /// **XHTML input is not validated.**
    /// Callers are responsible for ensuring that the given XHTML is conformant.
    ///
    /// # See Also
    /// - [`Self::xhtml_body`] for implicit XHTML document creation.
    /// - [`ResourceContent`] for details on providing data from memory (bytes/strings)
    ///   or the OS file system (paths).
    ///
    /// # Examples
    /// - Passing literal XHTML data:
    /// ```
    /// # use rbook::epub::EpubChapter;
    /// EpubChapter::new("Chapter 1").xhtml(
    ///     r#"<?xml version="1.0" encoding="UTF-8"?>
    ///     <html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
    ///         <head><title>Chapter 1</title></head>
    ///         <body><p>Hello World</p></body>
    ///     </html>"#
    /// );
    /// ```
    /// - Referencing a file stored on disk:
    /// ```
    /// # use std::path::PathBuf;
    /// # use rbook::epub::EpubChapter;
    /// EpubChapter::new("Chapter 2")
    ///     // The location where the resource will be stored within the EPUB.
    ///     .href("chapter_2.xhtml")
    ///     // The location of the source file on the OS file system.
    ///     .xhtml(PathBuf::from("local/data/chapters/c2.xhtml"));
    /// ```
    pub fn xhtml(mut self, xhtml: impl Into<ResourceContent>) -> Self {
        let entry = self.manifest_entry.get_or_insert_with(|| {
            // Manifest entry `id` temporarily empty
            let mut detached = DetachedEpubManifestEntry::new("");

            // Synchronize hrefs
            if let Some(toc_entry) = self.toc_entry.as_ref().and_then(|e| e.as_view().href_raw()) {
                detached.as_mut().set_href(toc_entry.as_str());
            }
            // NOTE - If there is no href:
            // - It may be provided later via `Self::href`.
            // - It there is no user-specified `href` upon insertion into `EpubEditor`,
            //   the href is automatically generated.
            detached
        });

        entry.as_mut().set_content(xhtml.into());
        self
    }

    /// Convenience method to set the XHTML `body` content as-is for end-user reading.
    ///
    /// **XHTML body input is not validated.**
    /// Callers are responsible for ensuring that the given XHTML is conformant.
    ///
    /// # Composition
    /// - `${EpubChapter::title}` is replaced with [`Self::title`] (XHTML escaped).
    /// - `${EpubChapter::xhtml_body}` is replaced with the given input.
    ///
    /// ```xhtml
    /// <?xml version="1.0" encoding="UTF-8"?>
    /// <html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
    /// <head>
    ///   <title>${EpubChapter::title}</title>
    /// </head>
    /// <body>
    /// ${EpubChapter::xhtml_body}
    /// </body>
    /// </html>
    /// ```
    ///
    /// # Note
    /// - Indentation, newlines, and whitespace in the body are preserved.
    /// - It is strongly recommended to not change [`Self::title`] after calling this method.
    ///   [`Self::title`] ***will not*** update the XHTML content.
    ///
    /// # See Also
    /// - [`Self::xhtml`] to set the entire XHTML document.
    ///
    /// # Examples
    /// ```
    /// # use rbook::epub::EpubChapter;
    /// EpubChapter::new("Chapter 1").xhtml_body(
    ///     "<h1>rbook c1</h1>\n\
    ///     <p>Paragraph 1</p>\n\
    ///     <p>Paragraph 2</p>"
    /// );
    /// ```
    pub fn xhtml_body(self, body: impl Into<Vec<u8>>) -> Self {
        let title = self
            .toc_entry
            .as_ref()
            .map(|toc_entry| toc_entry.as_view().label())
            .unwrap_or_default();

        let mut xhtml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
            \n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\
            \n<head>\
            \n  <title>{}</title>\
            \n</head>\
            \n<body>\n",
            quick_xml::escape::escape(title),
        )
            .into_bytes();

        xhtml.extend(body.into());
        xhtml.extend(b"\n</body>\n</html>");
        self.xhtml(xhtml)
    }

    /// Appends one or more [subchapters](EpubChapter) to this entry via the [`Many`] trait.
    ///
    /// # Examples
    /// ```
    /// # use rbook::ebook::toc::TocEntryKind;
    /// # use rbook::epub::EpubChapter;
    /// # const SAMPLE_XHTML: &[u8] = &[];
    /// EpubChapter::new("Volume I")
    ///     .xhtml(SAMPLE_XHTML)
    ///     .href("v1.xhtml")
    ///     .kind(TocEntryKind::Volume)
    ///     // Single entry
    ///     .children(
    ///         EpubChapter::new("I").href("v1c1.xhtml").xhtml(SAMPLE_XHTML),
    ///     )
    ///     // Batch entries
    ///     .children([
    ///         EpubChapter::new("II").href("v1c2.xhtml").xhtml(SAMPLE_XHTML),
    ///         EpubChapter::new("III").href("v1c3.xhtml").xhtml(SAMPLE_XHTML),
    ///         EpubChapter::new("IV").href("v1c4.xhtml").xhtml(SAMPLE_XHTML),
    ///         EpubChapter::new("V").href("v1c5.xhtml").xhtml(SAMPLE_XHTML),
    ///     ]);
    /// ```
    pub fn children(mut self, sub_chapter: impl Many<Self>) -> Self {
        self.sub_chapters.extend(sub_chapter.iter_many());
        self
    }
}
