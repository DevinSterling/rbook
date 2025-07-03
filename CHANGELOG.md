# Changelog

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

  | Implementor                          | Iterator Item                   |
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
- Refactor core traits (i.e., `Ebook`) with a greater detailed contract and shared interface for current and future formats.
- Greatly enhance Resource API for retrieval and analysis of an ebook's contents, 
  such as analyzing MIME type of resources in detail.
- Configurable `Epub` and `EpubReader` instances via `EpubSettings` and `EpubReaderSettings`, 
  enabling control over content order and strictness.
- Improved, faster, and more scalable version-agnostic parsing of EPUBs.
- Add `prelude` feature for convenient trait imports.
- Rename `multi-thread` feature to `threadsafe` which is now enabled by default.
  The new name further clarifies that an instance (i.e., `Epub`) may safely be shared between threads.
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
