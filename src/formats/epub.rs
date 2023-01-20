mod spine;
mod manifest;
mod guide;
mod metadata;
mod table_of_contents;

use lol_html::{element, text, doc_text, HtmlRewriter, Settings, Selector, ElementContentHandlers, DocumentContentHandlers};
use std::rc::Rc;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufReader, Read, Seek};
use std::path::{Path, PathBuf};
use std::fmt::{Debug, Formatter};

use crate::utility;
use crate::formats::{Ebook, EbookError};
use crate::archive::{Archive, ZipArchive, DirArchive};
use crate::formats::xml::{self, Element};
use crate::formats::xml::utility as xmlutil;
#[cfg(feature = "reader")]
use crate::reader::{ReaderError, Reader, Readable};
#[cfg(feature = "statistics")]
use crate::statistics::Stats;

pub use self::{
    metadata::Metadata,
    manifest::Manifest,
    guide::Guide,
    spine::Spine,
    table_of_contents::Toc,
};

// Location of .xml file that leads to the package.opf
const CONTAINER: &str = "META-INF/container.xml";

/// Electronic Publication (epub) format
///
/// Provides access to the following contents of an epub:
/// - [Metadata](Metadata)
/// - [Manifest](Manifest)
/// - [Spine](Spine)
/// - [Landmarks/Guide](Guide)
/// - [Table of Contents (toc)](Toc)
///
/// # Examples:
/// Basic usage:
/// ```
/// use rbook::Ebook;
///
/// // Creating an epub instance
/// let epub = rbook::Epub::new("example.epub").unwrap();
///
/// // Retrieving the title
/// println!("Title = {}", epub.metadata().title().value());
///
/// // Creating a reader instance
/// let mut reader = epub.reader();
///
/// // Printing the contents of each page
/// while let Some(content) = reader.next_page() {
///     println!("{content}")
/// }
/// ```
pub struct Epub {
    archive: RefCell<Box<dyn Archive>>,
    root_file: PathBuf,
    metadata: Metadata,
    manifest: Manifest,
    spine: Spine,
    guide: Guide,
    toc: Toc,
}

impl Epub {
    #[cfg(feature = "reader")]
    pub fn reader(&self) -> Reader {
        Reader::new(self, self.spine.elements().len())
    }

    /// Access ebook metadata such as author, title, date, etc.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Access all resources for the epub, such as images, files, etc.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Access the page order for the ebook
    pub fn spine(&self) -> &Spine {
        &self.spine
    }

    /// Access important structural portions of the ebook
    ///
    /// Primarily used by epub2. Access to epub3 landmarks is
    /// accessible using the [landmarks()](Toc::landmarks) method in [Toc](Toc).
    pub fn guide(&self) -> &Guide {
        &self.guide
    }

    /// Access the table of contents
    pub fn toc(&self) -> &Toc {
        &self.toc
    }

    /// Retrieve the cover image element from the [manifest](Manifest)
    ///
    /// # Examples
    /// Retrieving cover image raw data:
    /// ```
    /// # use rbook::Ebook;
    /// # let epub = rbook::Epub::new("example.epub").unwrap();
    /// // Retrieve the href from the cover image element
    /// let cover_href = epub.cover_image().unwrap().value();
    ///
    /// let cover_image_data = epub.read_bytes_file(cover_href).unwrap();
    /// ```
    pub fn cover_image(&self) -> Option<&Element> {
        match self.metadata.cover() {
            Some(cover_meta) => self.manifest.by_id(cover_meta.value()),
            None => self.manifest.by_property("cover-image"),
        }
    }

    /// Retrieve the root ".opf" file associated with the ebook.
    ///
    /// # Examples
    /// Basic Usage:
    /// ```
    /// # use rbook::Ebook;
    /// # use std::path::PathBuf;
    /// # let epub = rbook::Epub::new("example.epub").unwrap();
    /// let root_file = epub.root_file();
    ///
    /// assert_eq!(PathBuf::from("OEBPS/package.opf"), root_file);
    /// ```
    pub fn root_file(&self) -> PathBuf {
        self.root_file.to_path_buf()
    }

    /// Retrieve the root file directory of the ebook where
    /// resources are stored
    ///
    /// # Examples
    /// Basic Usage:
    /// ```
    /// # use rbook::Ebook;
    /// # use std::path::PathBuf;
    /// # let epub = rbook::Epub::new("example.epub").unwrap();
    /// let root_file_dir = epub.root_file_directory();
    /// assert_eq!(PathBuf::from("OEBPS"), root_file_dir);
    ///
    /// let root_file = root_file_dir.join("package.opf");
    /// assert_eq!(PathBuf::from("OEBPS/package.opf"), root_file);
    /// ```
    pub fn root_file_directory(&self) -> PathBuf {
        utility::get_parent_path(&self.root_file)
    }

    /// Retrieve the file contents.
    ///
    /// The given path is appended to the root file directory if it
    /// does not contain it. However, retrieving "META-INF/container.xml"
    /// is an exception. Please note that the root file directory
    /// varies between ebooks.
    ///
    /// # Examples:
    /// Basic usage:
    /// ```
    /// # use rbook::Ebook;
    /// # let epub = rbook::Epub::new("example.epub").unwrap();
    /// // Without providing the root file directory
    /// let content1 = epub.read_file("package.opf").unwrap();
    /// // Providing the root file directory
    /// let content2 = epub.read_file("OEBPS/package.opf").unwrap();
    ///
    /// assert_eq!(content1, content2)
    /// ```
    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<String, EbookError> {
        let path = self.parse_path(&path);
        self.archive.borrow_mut()
            .read_file(&path)
            .map_err(EbookError::Archive)
    }

    /// Retrieve the file contents in bytes.
    ///
    /// The given path is appended to the root file directory if it
    /// does not contain it. Please note that the root file directory
    /// varies between ebooks.
    ///
    /// # Examples:
    /// Basic usage:
    /// ```
    /// # use rbook::Ebook;
    /// # let epub = rbook::Epub::new("example.epub").unwrap();
    /// // Without providing the root file directory
    /// let content1 = epub.read_bytes_file("images/Art_insert1.jpg").unwrap();
    /// // Providing the root file directory
    /// let content2 = epub.read_bytes_file("OEBPS/images/Art_insert1.jpg").unwrap();
    ///
    /// assert_eq!(content1, content2)
    /// ```
    pub fn read_bytes_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>, EbookError> {
        let path = self.parse_path(&path);
        self.archive.borrow_mut()
            .read_bytes_file(&path)
            .map_err(EbookError::Archive)
    }

    // Transform a given path into a valid path if necessary
    // to traverse the contents of the ebook
    fn parse_path<'a, P: AsRef<Path>>(&self, path: &'a P) -> Cow<'a, Path> {
        let root_file_dir = utility::get_parent_path(&self.root_file);
        let path = path.as_ref();

        // if the given path is not the container and does not contain the
        // root file dir, concat the user supplied path to the root file dir
        if !(PathBuf::from(CONTAINER) == path || path.starts_with(&root_file_dir)) {
            Cow::Owned(root_file_dir.join(path))
        } else {
            Cow::Borrowed(path)
        }
    }

    fn build(mut archive: Box<dyn Archive>) -> Result<Self, EbookError> {
        // Parse "META-INF/container.xml"
        let content_meta_inf = archive.read_bytes_file(Path::new(CONTAINER))
            .map_err(EbookError::Archive)?;
        let root_file = parse_container(&content_meta_inf)?;

        // Get epub root file directory
        let root_file_dir = utility::get_parent_path(&root_file);

        // Parse "package.opf"
        let content_pkg_opf = archive.read_bytes_file(&root_file)
            .map_err(EbookError::Archive)?;
        let (metadata,
            manifest,
            spine,
            guide
        ) = parse_package(&content_pkg_opf)?;

        // Get toc.xhtml/ncx href value
        let toc_href = get_toc(&manifest)?.value();

        // Parse "toc.xhtml/ncx"
        let content_toc = archive.read_file(&root_file_dir.join(toc_href))
            .map_err(EbookError::Archive)?;
        let toc = parse_toc(&content_toc)?;

        Ok(Self {
            archive: RefCell::new(archive),
            root_file,
            metadata,
            manifest,
            spine,
            guide,
            toc,
        })
    }
}

impl Debug for Epub {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Epub")
            .field("root_file", &self.root_file)
            .field("metadata", &self.metadata)
            .field("manifest", &self.manifest)
            .field("spine",&self.spine)
            .field("landmarks", &self.guide)
            .field("toc", &self.toc)
            .finish()
    }
}

impl Ebook for Epub {
    type Format = Self;

    fn new<P: AsRef<Path>>(path: P) -> Result<Self, EbookError> {
        let metadata = utility::get_path_metadata(&path)?;

        // Unzip the file if it is not directory. If it is, the contents can
        // be accessed directly which makes using a zip file unnecessary.
        let archive: Box<dyn Archive> = if metadata.is_file() {
            let file = utility::get_file(&path)?;
            Box::new(ZipArchive::new(BufReader::new(file))?)
        } else {
            Box::new(DirArchive::new(&path)?)
        };

        Epub::build(archive)
    }

    fn read_from<R: Seek + Read + 'static>(reader: R) -> Result<Self::Format, EbookError> {
        Epub::build(Box::new(ZipArchive::new(reader)?))
    }
}

#[cfg(feature = "reader")]
impl Readable for Epub {
    fn navigate_str(&self, path: &str) -> Result<usize, ReaderError> {
        // Avoid freeing reference to elements while still in use
        let manifest_elements = self.manifest.elements();

        let manifest_element = manifest_elements.iter()
            .find(|element| element.value() == path)
            .ok_or_else(|| ReaderError::InvalidReference {
            cause: "Invalid path provided".to_string(),
            description: format!("Please ensure a manifest element's href \
                attribute references the path: '{path}'.")
            })?;

        // Get index of the spine element
        let spine_element_index = self.spine.elements()
            .iter()
            .position(|element| element.name() == manifest_element.name())
            .ok_or_else(|| ReaderError::InvalidReference {
                cause: "Manifest element is not referenced".to_string(),
                description: format!("Please ensure a spine element's idref attribute \
                references the manifest element with an id of: '{}'", manifest_element.name())
            })?;

        Ok(spine_element_index)
    }

    fn navigate(&self, index: usize) -> Result<String, ReaderError> {
        let spine_element = self.spine.elements().get(index)
            .ok_or_else(|| ReaderError::OutOfBounds {
                cause: format!("Provided index '{index}' is out of bounds"),
                description: "Please ensure the index is in bounds".to_string(),
            })?;

        let manifest_element = self.manifest.by_id(spine_element.name())
            .ok_or_else(|| ReaderError::InvalidReference {
                cause: "Invalid manifest reference".to_string(),
                description: "Please ensure all spine elements reference a \
                valid manifest element.".to_string(),
            })?;

        let file_content = self.read_file(manifest_element.value())
            .map_err(ReaderError::NoContent)?;

        Ok(file_content)
    }
}

#[cfg(feature = "statistics")]
impl Stats for Epub {
    fn count_total<F: Fn(&[u8]) -> Result<usize, EbookError>>(&self, f: F) -> usize {
        self.spine.elements()
            .iter()
            .filter_map(|element| self.manifest.by_id(element.name()))
            .filter_map(|element| self.read_bytes_file(element.value()).ok())
            .filter_map(|content| f(&content).ok())
            .sum()
    }

    fn try_count_total<F: Fn(&[u8]) -> Result<usize, EbookError>>(&self, f: F) -> Result<usize, EbookError> {
        self.spine.elements()
            .iter()
            .try_fold(0, |total, element| {
                let manifest_el = self.manifest.by_id(element.name())
                    .ok_or_else(|| EbookError::Parse {
                        cause: "Invalid manifest reference".to_string(),
                        description: "Please ensure all spine elements reference a \
                        valid manifest element.".to_string(),
                    })?;
                let content = self.read_bytes_file(manifest_el.value())?;
                let count = f(&content)?;

                Ok(total + count)
            })
    }

    fn count_chars(&self, data: &[u8]) -> Result<usize, EbookError> {
        let mut char_count: usize = 0;

        let char_handler = text!("body > *", |text| {
            char_count += text.as_str().len();

            Ok(())
        });

        parse_xhtml_data(
            vec![char_handler],
            vec![],
            data
        )?;

        Ok(char_count)
    }

    fn count_words(&self, data: &[u8]) -> Result<usize, EbookError> {
        let mut word_count: usize = 0;

        let text_handler = text!("body > *", |text| {
            word_count += text.as_str()
                .split(|character: char| !character.is_alphanumeric())
                .filter(|capture| !capture.is_empty())
                .count();

            Ok(())
        });

        parse_xhtml_data(
            vec![text_handler],
            vec![],
            data
        )?;

        Ok(word_count)
    }
}

fn parse_container(data: &[u8]) -> Result<PathBuf, EbookError> {
    let mut opf_location = String::new();

    let root_file_handler = element!("rootfile", |element| {
        // Although rare, multiple package.opf locations could
        // exist. Only accept first path, ignore all others
        if !opf_location.is_empty() { return Ok(()) }

        if let (Some(media_type), Some(full_path)) = (
            element.get_attribute("media-type"),
            element.get_attribute("full-path")
        ) {
            if media_type == "application/oebps-package+xml" {
                opf_location.push_str(&full_path);
            }
        }

        Ok(())
    });

    parse_xhtml_data(
        vec![root_file_handler],
        vec![],
        data
    )?;

    if opf_location.is_empty() {
        Err(EbookError::Parse {
            cause: "Missing .opf location".to_string(),
            description: "Please ensure that there is a valid 'rootfile' \
            that leads to the .opf file in 'META-INF/container.xml'.".to_string(),
        })
    } else {
        Ok(PathBuf::from(opf_location))
    }
}

fn parse_package(data: &[u8]) -> Result<(Metadata, Manifest, Spine, Guide), EbookError> {
    // Keep track of latest metadata entry
    let current_meta = RefCell::new(None);
    // Track contents
    let mut metadata_map: HashMap<String, Element> = HashMap::new(); // Metadata contents
    let mut item_map = HashMap::new(); // Manifest contents
    let mut itemref_vec = Vec::new(); // Spine contents
    let mut guide_vec = Vec::new(); // Guide contents (Epub 2 Only)
    let mut package_root = None; // Package element
    let mut spine_root = None; // Spine element

    // Stores the package and spine elements
    let parent_element_handler = element!("package, spine", |element| {
        let name = element.tag_name();
        let root = match name.as_str() {
            "package" => &mut package_root,
            _ => &mut spine_root,
        };

        root.replace(Element {
            name,
            attributes: xmlutil::copy_attributes(element.attributes()),
            value: String::new(),
            children: None,
        });

        Ok(())
    });

    let metadata_entry_handler = element!("metadata > *", |element| {
        let mut attributes = xmlutil::copy_attributes(element.attributes());
        let mut value = String::new();
        let mut name = element.tag_name();

        // Change name to the value of the name or
        // property attribute of a meta element
        match (
            xmlutil::take_attribute(&mut attributes, "property"),
            xmlutil::take_attribute(&mut attributes, "name"),
            xmlutil::take_attribute(&mut attributes, "content")
        ) {
            // Newer meta element
            (Some(property), _, _) => {
                name = property.value
            },
            // Legacy OPF2.0 meta element
            (_, Some(meta_name), Some(content)) => {
                name = meta_name.value;
                value = content.value;

                attributes.push(xml::Attribute {
                    name: "_rbook_legacy_feature".to_string(),
                    value: "OPF2 meta".to_string(),
                });
            },
            _ => (),
        }

        // Remove namespace
        if let Some((_, right)) = utility::split_where(&name, ':') {
            name = right.to_string();
        }

        // Add element to metadata
        let mut meta = Element {
            name,
            attributes,
            value,
            children: None,
        };

        // If the entry has an id attribute, it most
        // likely has children metadata further describing it
        let key = if let Some(id) = element.get_attribute(xml::ID) {
            meta.children = Some(Vec::new());
            id
        }
        // If there is no id, give the meta entry a temporary id
        // to avoid potential value replacements in the hash map.
        else {
            format!("_rbook_{}", metadata_map.len())
        };

        // Add child metadata to parent metadata
        if let Some(refines) = element.get_attribute("refines") {
            let id = refines.replace('#', "");
            let children = metadata_map.get_mut(&id)
                .and_then(|parent| parent.children.as_mut());

            if let Some(children) = children {
                children.push(meta);

                if let Some(last_child) = children.last_mut() {
                    current_meta.borrow_mut().replace(last_child as *mut Element);
                }
            }
        }
        // Add new metadata entry
        else {
            metadata_map.insert(key.to_string(), meta);

            if let Some(last_entry) = metadata_map.get_mut(&key) {
                current_meta.borrow_mut().replace(last_entry as *mut Element);
            }
        }

        Ok(())
    });

    // Capture text from "dc:*" and "meta" elements. Used instead
    // of text!(...) to obtain text values encased between "meta" tags
    let metadata_text_value_handler = doc_text!(|text| {
        let value = text.as_str().trim().to_string();

        // Ignore empty chunks/strings
        if value.is_empty() { return Ok(()) }

        // Add missing metadata value to current metadata entry
        // TODO: ensure function only runs when text is encased in "metadata" tags
        if let Some(meta_entry) = current_meta.borrow_mut().take() {
            // This should be guaranteed to have a valid address
            unsafe { (*meta_entry).value = value }
        }

        Ok(())
    });

    let manifest_handler = element!("item", |element| {
        let mut attributes = xmlutil::copy_attributes(element.attributes());

        // the name of manifest items will be the value of its id attribute
        let (name, value) = match (
            xmlutil::take_attribute(&mut attributes, xml::ID),
            xmlutil::take_attribute(&mut attributes, xml::HREF)
        ) {
            (Some(id), Some(href)) => (id.value, href.value),
            _ => return Ok(()),
        };

        item_map.insert(
            name.to_string(),
            Element {
                name,
                attributes,
                value,
                children: None,
            }
        );

        Ok(())
    });

    let spine_handler = element!("itemref", |element| {
        let mut attributes = xmlutil::copy_attributes(element.attributes());

        // the name of spine items will be the value of its idref attribute
        let name = match xmlutil::take_attribute(&mut attributes, "idref") {
            Some(idref) => idref.value,
            _ => return Ok(())
        };

        itemref_vec.push(Element {
            name,
            attributes,
            value: String::new(),
            children: None,
        });

        Ok(())
    });

    // Epub 2 feature
    let guide_handler = element!("reference", |element| {
        let mut attributes = xmlutil::copy_attributes(element.attributes());

        let (name, value) = match (
            xmlutil::take_attribute(&mut attributes, "title"),
            xmlutil::take_attribute(&mut attributes, xml::HREF)
        ) {
            (Some(title), Some(href)) => (title.value, href.value),
            _ => return Ok(()),
        };

        guide_vec.push(Element {
            name,
            attributes,
            value,
            children: None,
        });

        Ok(())
    });

    parse_xhtml_data(
        vec![
            parent_element_handler,
            metadata_entry_handler,
            manifest_handler,
            spine_handler,
            guide_handler,
        ],
        vec![metadata_text_value_handler],
        data
    )?;

    // Finalize package:
    // Check if the package references a valid unique identifier and contains the epub version
    let package_root = is_valid_package(package_root, &metadata_map)?;

    // Finalize spine:
    let spine_root = is_valid_spine(spine_root, itemref_vec)?;

    // Finalize metadata:
    // Transfer metadata contents to new hashmap with meta categories
    let metadata_map = categorize_metadata(metadata_map);
    // Check if metadata contains the required contents
    is_valid_metadata(&metadata_map)?;

    Ok((
        Metadata::new(package_root, metadata_map),
        Manifest(item_map), // Add properties
        Spine(spine_root),
        Guide(guide_vec),
    ))
}

fn is_valid_package(package: Option<Element>, metadata: &HashMap<String, Element>) -> Result<Element, EbookError> {
    package.filter(|pkg| {
        pkg.get_attribute("unique-identifier")
            // Check if package contains a valid reference to a unique identifier
            .map(|attr| metadata.contains_key(attr.value()))
            // Check if package contains an epub version attribute
            .unwrap_or(false) && pkg.contains_attribute("version")
    }).ok_or(EbookError::Parse {
        cause: "Required attributes are missing or invalid uid reference".to_string(),
        description: "The package element is missing the 'unique-identifier' \
                or 'version' attribute. Please ensure 'unique-identifier' \
                references a valid identifier. This can be fixed in the .opf \
                file".to_string(),
    })
}

fn is_valid_spine(spine: Option<Element>, children: Vec<Element>) -> Result<Element, EbookError> {
    spine.map(|mut spine| {
        spine.children.replace(children);
        spine
    }).ok_or(EbookError::Parse {
        cause: "Required element is missing".to_string(),
        description: "Please ensure the 'spine' element exists in the .opf file".to_string(),
    })
}

fn is_valid_metadata(metadata: &HashMap<String, Vec<Element>>) -> Result<(), EbookError> {
    let required_keys = ["title", "identifier", "language"];
    let missing_key = required_keys.into_iter()
        .find(|key| !metadata.contains_key(*key));

    match missing_key {
        Some(key) => Err(EbookError::Parse {
            cause: format!("Required metadata '{key}' is missing"),
            description: "Please ensure the required metadata is not absent in \
             the .opf file.".to_string()
        }),
        None => Ok(())
    }
}

// Initially the keys in the map are id references for elements.
// Return a new map that has meta categories as the key instead.
fn categorize_metadata(metadata: HashMap<String, Element>) -> HashMap<String, Vec<Element>> {
    let mut categorized_metadata: HashMap<String, Vec<Element>> = HashMap::new();

    for (_, meta_element) in metadata {
        // The name of each element is a meta category,
        // such as "dc:identifier", there can be one or more
        let category = meta_element.name();

        // If the new map does not contain the category, add it
        // and push the current element into its category
        categorized_metadata.entry(category.to_string())
            .or_default()
            .push(meta_element);
    }

    categorized_metadata
}

fn parse_toc(mut data: &str) -> Result<Toc, EbookError> {
    // Keep track of latest nav element entry
    let parent_stack = Rc::new(RefCell::new(Vec::new()));
    let current_nav_group = Rc::new(RefCell::new(Vec::new()));
    let nav_groups = Rc::new(RefCell::new(HashMap::new()));

    // TODO: Temporary work around for a dependency bug at the moment
    // Bug: If the parser encounters a script element in the head,
    // such as "<script src="../to/file.js" type="text/javascript"/>",
    // then the parser will fail to identify all further elements
    if let Some(index) = data.find("<body") {
        data = &data[index..];
    }

    // nav group entry
    let nav_group_handler = element!("nav, navMap, pageList", |element| {
        let element_name = element.tag_name();
        let mut attributes = xmlutil::copy_attributes(element.attributes());

        let parent_stack = Rc::clone(&parent_stack);
        let current_nav_group = Rc::clone(&current_nav_group);
        let groups = Rc::clone(&nav_groups);
        element.on_end_tag(move |_| {
            let nav_group_name = match xmlutil::take_attribute(&mut attributes, "epub:type") {
                Some(nav_type) => nav_type.value,
                // If the element is pageList
                None if element_name == "pageList" => "page-list".to_string(),
                // Default the group name to "table of contents" (toc)
                _ => "toc".to_string(),
            };

            // Clear stack
            parent_stack.borrow_mut().clear();

            // Add elements to parent nav element
            groups.borrow_mut().insert(
                nav_group_name.to_string(),
                Element {
                    name: nav_group_name,
                    attributes,
                    value: String::new(),
                    children: Some(current_nav_group.replace(Vec::new())),
                }
            );

            Ok(())
        })?;

        Ok(())
    });

    // create new entry nav element
    let nav_entry_handler = element!("li, navPoint, pageTarget", |element| {
        parent_stack.borrow_mut().push(Element {
            name: String::new(),
            attributes: xmlutil::copy_attributes(element.attributes()),
            value: String::new(),
            children: None,
        });

        // Handle end tag event
        let parent_stack = Rc::clone(&parent_stack);
        let toc = Rc::clone(&current_nav_group);
        element.on_end_tag(move |_| {
            let mut stack = parent_stack.borrow_mut();

            match (stack.pop(), stack.last_mut()) {
                // Nav element has a parent
                (Some(nav_entry), Some(nav_parent)) => {
                    match nav_parent.children.as_mut() {
                        Some(children) => children.push(nav_entry),
                        None => nav_parent.children = Some(vec![nav_entry]),
                    }
                },
                // Nav element does not have a parent
                (Some(nav_entry), _) => toc.borrow_mut().push(nav_entry),
                _ => ()
            }

            Ok(())
        })?;

        Ok(())
    });

    // Set the value of the entry nav element to the href
    let nav_content_handler = element!("a, span, content", |element| {
        // Transfer attributes and obtain href
        if let Some(nav_entry) = parent_stack.borrow_mut().last_mut() {
            for attribute in xmlutil::copy_attributes(element.attributes()) {
                match attribute.name() {
                    "href" | "src" => nav_entry.value = attribute.value,
                    _ => nav_entry.attributes.push(attribute),
                }
            }
        }

        Ok(())
    });

    // Set the name of the entry nav element to the nav label
    let nav_text_handler = text!("a, span, text", |text| {
        let text = text.as_str().trim();

        // Ignore empty chunks/strings
        if !text.is_empty() {
            if let Some(nav_entry) = parent_stack.borrow_mut().last_mut() {
                nav_entry.name = text.to_string();
            }
        }

        Ok(())
    });

    parse_xhtml_data(
        vec![
            nav_group_handler,
            nav_entry_handler,
            nav_content_handler,
            nav_text_handler,
        ],
        vec![],
        data.as_bytes()
    )?;

    is_valid_toc(&nav_groups.borrow())?;

    Ok(Toc(nav_groups.take()))
}

fn is_valid_toc(toc: &HashMap<String, Element>) -> Result<(), EbookError> {
    if toc.contains_key("toc") {
        Ok(())
    } else {
        Err(EbookError::Parse {
            cause: "Missing toc element".to_string(),
            description: "The nav or navMap element for the toc is \
            missing. Please ensure it exists.".to_string(),
        })
    }
}

// Helper functions
fn parse_xhtml_data(
    element_content_handlers: Vec<(Cow<Selector>, ElementContentHandlers)>,
    document_content_handlers: Vec<DocumentContentHandlers>,
    data: &[u8]
) -> Result<(), EbookError> {
    let mut reader = HtmlRewriter::new(
        Settings {
            element_content_handlers,
            document_content_handlers,
            ..Settings::default()
        },
        |_: &[u8]| ()
    );

    // Convert data to utf-8 if necessary and start parsing
    match reader.write(&utility::to_utf8(data)) {
        Ok(_) => Ok(()),
        Err(error) => Err(EbookError::Parse {
            cause: "Parse Error".to_string(),
            description: format!("An error occurred while parsing: {error}"),
        }),
    }
}

fn get_toc(manifest: &Manifest) -> Result<&Element, EbookError> {
    // Attempt to retrieve newer toc format first
    manifest.by_property("nav")
        // Fallback to older toc format
        .or_else(|| manifest.by_media_type("application/x-dtbncx+xml"))
        .ok_or(EbookError::Parse {
            cause: "Missing table of contents (toc)".to_string(),
            description: "The toc element cannot be found within the \
            manifest. Please ensure there is a valid element that references \
            the table of contents file".to_string(),
        })
}