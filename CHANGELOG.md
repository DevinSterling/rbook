# Changelog
## 0.7.2 (2025-02-28)
### Additions　**＋**
- New method `EpubEditor::author` to simplify appending authors.
  This avoids callers having to manually specify the marc:relators `aut` code for `dc:creator` metadata entries.
- New method `Prefixes::get_uri` to simplify retrieving the URI of a `Prefix`.

### Fixes　**✓**
- Fix `EpubContributor::roles` incorrectly providing both EPUB 3 refinements roles and the EPUB 2 legacy `opf:role`.
  Now prioritizes EPUB 3 refinements over the legacy `opf:role` attribute.

### Changes　**⟳**
- Update `wasm-bindgen-test` dev dependency: 0.3.63 → 0.3.64
- Refine documentation for enhanced clarity.

## 0.7.1 (2025-02-26)
### Additions　**＋**
- New method `EpubEditor::modified_now` to simplify modification workflows
  by setting the modified date to the current datetime.

### Changes　**⟳**
- Update `wasm-bindgen-test` dev dependency: 0.3.62 → 0.3.63
- Refine documentation for enhanced clarity.

## 0.7.0 (2025-02-24)
### Major Release: Write Support
This release expands `rbook` into a read/write library,
introducing a comprehensive API for creating EPUBs from scratch and modifying
existing ones, including metadata, manifest, spine, and table of contents.

The summary below highlights the key changes.
All new APIs and examples are documented at: https://docs.rs/rbook/latest/rbook/

### Additions　**＋**
- New feature flag: `write` (enabled by default).
  - Adds support for saving `Epub` instances to disk/memory.
  - Adds fluent builders to greatly ease creation and modification (e.g., `EpubEditor`, `EpubWriteOptions`).
  - Adds all mutation APIs (`_mut` accessors).
- New `Epub` methods and associated functions:
  - `new`: Creates a new empty in-memory EPUB.
  - `builder`: Returns an `EpubEditor` to build an EPUB from scratch.
  - `edit`: Returns an `EpubEditor` to modify an existing EPUB.
  - `write`: Returns `EpubWriteOptions` to configure how to write an EPUB to disk/memory.
  - New `_mut` accessors: 
    - `metadata_mut`, `manifest_mut`, `spine_mut`, `toc_mut`
- New **Mutator** types for fine-grain control:
  - `EpubMetadataMut`, `EpubMetaEntryMut`, `EpubRefinementsMut`
  - `EpubManifestMut`, `EpubManifestEntryMut`
  - `EpubSpineMut`, `EpubSpineEntryMut`
  - `EpubTocMut`, `EpubTocEntryMut`
  - `AttributesMut`, `PropertiesMut`
- New **Detached** types for constructing entries independent of an `Epub`:
  - `DetachedEpubMetaEntry` + associated `marker` types for type-safety
  - `DetachedEpubManifestEntry`
  - `DetachedEpubSpineEntry`
  - `DetachedEpubTocEntry`
- Helper Traits:
  - `Many`: Enables methods to accept single items, arrays, or vectors uniformly.
  - `IntoOption`: Simplifies passing optional arguments (e.g., `id("xyz")` vs `id(Some("xyz"))`).
- Core ebook trait methods are now inherent on the concrete EPUB types.
  Importing traits is no longer required.

### Changes　**⟳**
- Update `quick-xml` dependency: 0.39.0 → 0.39.2
- Update `zip` dependency: 7.4.0 → 8.1.0
- Update `wasm-bindgen-test` dev dependency: 0.3.58 → 0.3.62
- Refine documentation for enhanced clarity.
- `EpubOpenOptions::strict` is now **`false`** by default.
- Optimization: Reduced the memory footprint of internal metadata entry structures.
- `EpubReaderOptions` is now generic, supporting both one-shot and reusable patterns.
- Rename `EpubOpenOptions::store_all` to `EpubOpenOptions::retain_variants` for clarity.
- Rename `EpubFormatError` to `EpubError`.
- Rename `EpubReaderBuilder` to `EpubReaderOptions<&Epub>`.
- Rename `ResourceKind::get_param` to `ResourceKind::by_param` for API consistency.
- Rename `Manifest::by_resource_kind` to `Manifest::by_kind`.
- Rename `Manifest::entries` to `Manifest::iter`.
- Rename `ManifestEntry::resource_kind` to `ManifestEntry::kind`.
- Rename `Metadata::entries` to `Metadata::iter`.
- Rename `Metadata::publication_date` to `Metadata::published`.
- Rename `Metadata::modified_date` to `Metadata::modified`.
- Rename `Spine::by_order` to `Spine::get`.
- Rename `Spine::entries` to `Spine::iter`.
- Rename `Toc::kinds` to `Toc::iter`.
- Rename `ReaderError::MalformedEbook` to `ReaderError::Format` for clarity.
- Merge `Manifest::by_resource_kinds` into `Manifest::by_kind`.
- Merge traits `TocEntry` and `TocEntryChildren` for ease-of-use.
- Implement `Copy` for `TocEntryKind`.
- Methods returning `Attributes` return `&Attributes` instead.
- Methods returning `Properties` return `&Properties` instead.
- `Toc::kinds` return type Item changed from `(&TocEntryKind, impl TocEntry)` to `impl TocEntry`.
  If the kind is needed, see `TocEntry::kind`. 

### Removals　**−**
- Remove all deprecated APIs.
- Remove struct `EpubReaderBuilder` as it is now unified with `EpubReaderOptions`.
- Remove method `ManifestEntry::key` as it served no purpose.
- Remove method `preferred_landmarks` and `preferred_page_list` from `EpubOpenOptions`.
- Remove enum variant `FormatError::InvalidUtf8`.
- Remove enum variant `ReaderError::InvalidEbookContent`.
- Remove `TocEntry::order` in favor of `TocEntry::flatten` paired with `Iterator::enumerate`,
  ensuring the order remains accurate after ToC modifications.

## 0.6.12 (2025-02-06)
## Additions　**＋**
- Clarify MSRV as `1.88.0`

### Changes　**⟳**
- Update `thiserror` dependency: 2.0.17 → 2.0.18
- Update `quick-xml` dependency: 0.38.4 → 0.39.0
- Update `zip` dependency: 7.0.0 → 7.4.0
- Update `wasm-bindgen-test` dev dependency: 0.3.56 → 0.3.58

## 0.6.11 (2025-02-05)
### Fixes　**✓**
- Fix compilation error caused by `zip` dependency;
  Cow `AsRef` ambiguity in `ResourceKind::as_static` from transitive `typed_path` dependency.
  \[[#5](https://github.com/DevinSterling/rbook/pull/5)]

### Changes　**⟳**
- Refine documentation for enhanced clarity.
- Internal refactoring in preparation for the upcoming write/modify API (v0.7.0).

## 0.6.10 (2025-12-20)
### Additions　**＋**
- New methods for `Href`:
  - `parent`: Retrieve the parent directory.
  - `extension`: Retrieve the file extension (See also: `ManifestEntry::resource_kind`).
- New methods for `EpubManifest`:
  - `scripts`: Iterate over all JavaScript resources.
  - `styles`: Iterate over all CSS stylesheets.
  - `fonts`: Iterate over all fonts (including legacy MIMEs, such as `application/font-woff`).
  - `audio`: Iterate over all audio resources.
  - `video`: Iterate over all video resources.
- New options for `EpubOpenOptions` for speed/space optimization:
  - `skip_metadata`: Skip parsing metadata.
  - `skip_manifest`: Skip parsing the manifest.
  - `skip_spine`: Skip parsing the spine.
  - `skip_toc`: Skip parsing `.ncx/.xhtml` table of contents files.
- New enum variant `EpubFormatError::InvalidHref` to indicate an href is malformed 
  (e.g., not percent-encoded).
- Implement `From<EpubManifestEntry>` for `Resource`.

### Fixes　**✓**
- Fix regression from `0.6.7` disallowing self-closing `<dc:*>` metadata elements
  when `EpubOpenOptions::strict` is disabled.
  \[[#2](https://github.com/DevinSterling/rbook/pull/2)]

### Changes　**⟳**
- Update `zip` dependency: 6.0.0 → 7.0.0
- Refine documentation for enhanced clarity.
- Improve parser resilience when `EpubOpenOptions::strict` is disabled.
- Internal refactoring in preparation for the upcoming write/modify API.
- Refactor test cases + add new test fixtures.

## 0.6.9 (2025-12-01)
### Additions　**＋**
- Add `example.epub` test file to `tests/ebooks` (No longer generated by `build.rs`).

### Changes　**⟳**
- Refine documentation for enhanced clarity.
- Update `thiserror` dependency: 2.0.12 → 2.0.17
- Update `percent-encoding` dependency: 2.3.1 → 2.3.2
- Update `quick-xml` dependency: 0.38.0 → 0.38.4
- Update `wasm-bindgen-test` dev dependency: 0.3.50 → 0.3.56

### Fixes　**✓**
- Fix `EpubVersion` `Ord` implementation to use the underlying numeric `Version` value rather than the enum variant.
- Fix `Epub` `PartialEq` implementation to compare all structural elements 
  (e.g., manifest, spine, toc) rather than just metadata.
- Fix `PartialEq` implementation for metadata views (e.g., `EpubTitle`, `EpubIdentifier`) 
  which incorrectly returned `true` when compared against an `EpubMetaEntry`.

### Removals　**−**
- Remove `build.rs` and associated build dependencies (`zip`, `zip-extensions`) to reduce compile times for users. 
  While useful for generating zipped test epubs, build scripts are executed for all, 
  incurring unnecessary overhead when used solely for test generation.

## 0.6.8 (2025-11-28)
### Additions　**＋**
- New associated function `Epub::options` to open an `Epub` with specific options (Inspired by `std::fs::File::options`).
- New method `Epub::reader_builder` to build an `EpubReader` with specific options.
- Improve robustness of `EpubMetadata::modified_date` and `EpubMetadata::publication_date`.
  These methods now support inferring from the `opf:event` attributes on `<dc:date>` elements.
- Implement `Display` for `TextDirection`.

### Changes　**⟳**
- Refine documentation for enhanced clarity.
- Rename `EpubSettings` to `EpubOpenOptions`. (`EpubSettings` is now a deprecated type alias)
- Rename `EpubReaderSettings` to `EpubReaderOptions`. (`EpubReaderSettings` is now a deprecated type alias)

### Fixes　**✓**
- Fix EPUB 2 cover images not conveniently retrievable from `EpubManifest::cover_image`.
- Fix `preferred_page_list` incorrectly set in `EpubOpenOptions::preferred_page_list`.
- Fix `preferred_landmarks` and `preferred_page_list` from `EpubOpenOptions` 
  not being correctly applied in `EpubToc`.
- Fix `IndexCursor` (for `EpubReader`) incorrectly incrementing to index `0` when `len` is `0`.

### Deprecations　**−**
- All public fields of `EpubSettings` and `EpubReaderSettings` 
  are now deprecated in favor of their respective builder methods.
- Deprecated as `Epub::options` is now preferred:
  - `EpubSettingsBuilder`
  - `EpubSettingsBuilder::build`
  - `EpubSettings::builder`
  - `Epub::open_with`
  - `Epub::read`
- Deprecated as `Epub::reader_builder` is now preferred:
  - `EpubReaderSettingsBuilder`
  - `EpubReaderSettingsBuilder::build`
  - `EpubReaderSettings::builder`
  - `Epub::reader_with`

## 0.6.7 (2025-11-27)
### Additions　**＋**
- Add support for EPUB 3 metadata `<link/>` elements.
- New *non-exhaustive* enum `EpubMetaEntryKind` to determine type of metadata entries 
  (e.g., `<dc:*>` (Dublin Core), `<meta>`, `<link>`).
- New struct `EpubLink` to access `<link>`-associated fields conveniently (e.g., `href`, `rel`, `properties`).
- New method `EpubMetadata::links` to retrieve all non-refining links.
- New method `EpubMetaEntry::kind` to determine the `EpubMetaEntryKind` of a metadata entry.
- New method `EpubMetaEntry::as_link` to retrieve an `EpubLink` view.
- New enum variant `EpubFormatError::MissingValue` to indicate if the required inner text of an element is absent.
- Implement `Display` for `Href`.

### Changes　**⟳**
- Update `zip` dependency: 4.3.0 → 6.0.0
- Refine documentation for enhanced clarity.

### Fixes　**✓**
- Fix where authors explicitly set refining metadata elements with duplicate `display-seq` 
  (Display Sequence) values.

## 0.6.6 (2025-07-13)
### Additions　**＋**
- New `name` method for `Href` to retrieve the encapsulated filename.
- Implement `From<String>` and `From<Cow<'a, str>>` for `ResourceKey`.
- Implement `From<Cow<'a, str>>` for `ResourceKind`.

### Changes　**⟳**
- Refine documentation for enhanced clarity.

## 0.6.5 (2025-07-11)
### Additions　**＋**
- New `by_id` method for `EpubMetadata` and `EpubSpine` to retrieve entries by their id.
- Implement `PartialEq<EpubMetaEntry>` for:
  - `EpubIdentifier`
  - `EpubTitle`
  - `EpubTag`
  - `EpubContributor`
  - `EpubLanguage`

### Changes　**⟳**
- Update `zip` dependency: 4.2.0 → 4.3.0
- Refine documentation for enhanced clarity.

## 0.6.4 (2025-07-09)
### Additions　**＋**
- New `max_depth` and `total_len` methods for `TocEntry`.
- Regarding EPUB, when a `title-type` of `main` is absent, rbook now infers the main `Title`
  by selecting the `<dc:title>` with the highest precedence (lowest display order).

  This guarantees consistent main title identification across all EPUBs.

### Changes　**⟳**
- Update `quick-xml` dependency: 0.37.5 → 0.38.0
- Refine documentation for enhanced clarity.

## 0.6.3 (2025-07-04)
### Additions　**＋**
- New `read_str` and `read_bytes` methods for resource retrieval directly from `ManifestEntry` instances.

### Changes　**⟳**
- Refine documentation for enhanced clarity.
- Refactor/simplify internals.

## 0.6.2 (2025-07-03)
### Additions　**＋**
- Implement `From<Href>` for `ResourceKey`.
- Expand test coverage.

### Changes　**⟳**
- Refine documentation for enhanced clarity.
- Refactor/simplify internals.

### Fixes　**✓**
- Fix shorter than expected lifetimes on references/instances returned by: 
  - `Ebook`
  - `Manifest`
  - `Metadata`
  - `Spine`
  - `Toc`
  - `Reader`
  - `ReaderContent`
  - `Href`
  - `Name`

## 0.6.1 (2025-07-01)
### Additions　**＋**
- Implement `IntoIterator` for:

  | Implementer                          | Iterator Item                   |
  |--------------------------------------|---------------------------------|
  | `EpubSpine`/`&EpubSpine`             | `EpubSpineEntry`                |
  | `EpubManifest`/`&EpubManifest`       | `EpubManifestEntry`             |
  | `EpubToc`/`&EpubToc`                 | `(&TocEntryKind, EpubTocEntry)` |
  | `EpubTocChildren`/`&EpubTocChildren` | `EpubTocEntry`                  |
  | `EpubRefinements`/`&EpubRefinements` | `EpubMetaEntry`                 |
  | `Attributes`/`&Attributes`           | `Attribute`                     |
  | `Properties`/`&Properties`           | `&str`                          |

### Changes　**⟳**
- Update `zip` dependency: 3.0.0 → 4.2.0
- Refine documentation for enhanced clarity.
- Refactor/simplify internals.

## 0.6.0 (2025-06-27)
### Structural Overhaul　**⟳**
This release introduces a major structural overhaul. 
The summary below highlights the key changes.

### Additions　**＋**
- New, more expressive models (`Spine`, `Metadata`, `Manifest`, `Toc`, `Attributes`, `Properties`, etc.), 
  self-documenting types, and improved documentation.
- Refactor core traits (e.g., `Ebook`) with a greater detailed contract and shared interface for current and future formats.
- Greatly enhance Resource API for retrieval and analysis of an ebook's contents, 
  such as analyzing MIME type of resources in detail.
- Configurable `Epub` and `EpubReader` instances via `EpubSettings` and `EpubReaderSettings`, 
  enabling control over content order and strictness.
- Improved, faster, and more scalable version-agnostic parsing of EPUBs.
- Add `prelude` feature for convenient trait imports.
- Rename `multi-thread` feature to `threadsafe` which is now enabled by default.
  The new name further clarifies that an instance (e.g., `Epub`) may safely be shared between threads.
- Replace `Vec<_>` return types with iterators for greater control and efficiency.
- Hrefs are now automatically resolved to simplify resource access.
- More detailed errors in returned results pinpointing where problems originate from.
- `wasm32-unknown-unknown` support.
- All additions are reflected in the documentation: https://docs.rs/rbook/latest/rbook/

### Removals　**−**
- Remove parent retrieval from children in tree-like structures, such as when navigating the table of contents. 
  The previous `Rc`/`Arc` + `Weak` approach works, although impedes future `Epub` mutability and write-support.
- Remove the `statistics` API (word and character count), as the implementation did not meet quality expectations.
- Remove the CSS selector-like `Find` API as the internal structure no longer represents a complete DOM-like tree.
- Merge `Guide` into `EpubToc` to reduce redundancy.
