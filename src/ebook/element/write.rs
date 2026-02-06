use crate::ebook::element::{Attribute, Attributes, Properties};
use crate::util::str::StringExt;

impl Properties {
    /// Returns [`Some`] if populated, otherwise [`None`].
    pub(crate) fn as_option_str(&self) -> Option<&str> {
        (!self.0.is_empty()).then_some(self.0.as_str())
    }

    /// Inserts one or more properties, returning `true` if at least one new property was added.
    /// `false` is returned if all given properties are already present.
    ///
    /// The given input is split by whitespace and each property is inserted individually.
    ///
    /// # Examples
    /// - Inserting multiple properties:
    /// ```
    /// # use rbook::ebook::element::Attribute;
    /// # let mut attribute = Attribute::new("properties", "");
    /// # let mut properties = attribute.as_properties_mut();
    ///
    /// // Inserting multiple properties
    /// # // `assert_eq` is used over `assert` as it is more readable
    /// assert_eq!(true, properties.insert("nav scripted"));
    /// assert_eq!("nav scripted", properties.as_str());
    ///
    /// // Inserting an existing and new property
    /// // Returns `true` as `cover` is new (`nav is skipped)
    /// assert_eq!(true, properties.insert("nav cover"));
    /// assert_eq!("nav scripted cover", properties.as_str());
    ///
    /// // Inserting an already existing property
    /// // Returns `false` as `scripted` is already present
    /// assert_eq!(false, properties.insert("scripted"));
    /// assert_eq!("nav scripted cover", properties.as_str());
    /// ```
    pub fn insert(&mut self, properties: &str) -> bool {
        let mut any_inserted = false;

        for property in properties.split_whitespace() {
            if !self.has_property(property) {
                any_inserted = true;

                if !self.0.is_empty() {
                    self.0.push(' ');
                }
                self.0.push_str(property);
            }
        }
        any_inserted
    }

    /// Removes one or more properties, returning `true` if at least one property was removed.
    /// `false` is returned if all given properties are not present.
    ///
    /// The given input is split by whitespace and each property is removed individually.
    ///
    /// # Examples
    /// - Removing multiple properties:
    /// ```
    /// # use rbook::ebook::element::Attribute;
    /// # let mut attribute = Attribute::new("properties", "nav scripted cover");
    /// # let mut properties = attribute.as_properties_mut();
    ///
    /// // Initial state
    /// assert_eq!("nav scripted cover", properties.as_str());
    ///
    /// assert_eq!(false, properties.remove("script"));
    /// assert_eq!("nav scripted cover", properties.as_str());
    ///
    /// // Removing an existing property
    /// // Returns `true` as the property was present and now removed
    /// assert_eq!(true, properties.remove("scripted"));
    /// assert_eq!("nav cover", properties.as_str());
    ///
    /// // Removing multiple properties
    /// assert_eq!(true, properties.remove("nav scripted cover"));
    /// assert!(properties.is_empty());
    /// ```
    pub fn remove(&mut self, properties: &str) -> bool {
        if self.is_empty() {
            return false;
        }
        let mut any_removed = false;

        for property in properties.split_whitespace() {
            // Check if the property to remove exists
            // - In most, if not nearly all cases, `self` is empty or contains one property
            let match_pos = self.0.match_indices(property).find(|&(start, _)| {
                let end = start + property.len();
                // Check spacing to ensure a proper space-separated match
                let leading_ok = start == 0 || self.0.as_bytes()[start - 1] == b' ';
                let trailing_ok = end == self.0.len() || self.0.as_bytes()[end] == b' ';
                leading_ok && trailing_ok
            });

            if let Some((start, _)) = match_pos {
                let end = start + property.len();
                any_removed = true;

                if end < self.0.len() {
                    // Has a trailing space: "xyz "
                    self.0.drain(start..=end);
                } else if start > 0 {
                    // Has a leading space: " xyz"
                    self.0.drain(start - 1..end);
                } else {
                    // The only property
                    self.0.clear();
                }
            }
        }
        any_removed
    }

    /// Removes all properties.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl Attribute {
    /// Creates a new attribute with the given [`name`](Self::name) and [`value`](Self::value).
    ///
    /// The given input has their leading and trailing whitespace trimmed in-place.
    ///
    /// The value is stored as plain text (e.g. `"1 < 2 & 3"`)
    /// and is XML-escaped automatically during [writing](crate::Epub::write).
    ///
    /// # Examples
    /// - Creating an attribute:
    /// ```
    /// # use rbook::ebook::element::Attribute;
    /// let attribute = Attribute::new("rbook:val", " 123 ");
    /// let name = attribute.name();
    /// assert_eq!("rbook:val", name);
    /// assert_eq!(Some("rbook"), name.prefix());
    /// assert_eq!("val", name.local());
    /// assert_eq!("123", attribute.value());
    ///
    /// let into_attribute: Attribute = (" val ", "456").into();
    /// assert_eq!("val", into_attribute.name());
    /// assert_eq!("456", into_attribute.value());
    /// ```
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::create(name, value)
    }

    /// Sets the attribute value and returns the previous value.
    ///
    /// The given `value` has its leading and trailing whitespace trimmed in-place.
    ///
    /// The value is stored as plain text (e.g. `"1 < 2 & 3"`)
    /// and is XML-escaped automatically during [writing](crate::Epub::write).
    ///
    /// # Examples
    /// - Setting an attribute value:
    /// ```
    /// # use rbook::ebook::element::Attribute;
    /// # fn main() {
    /// let mut attribute = Attribute::new("val", "123");
    /// assert_eq!("123", attribute.value());
    ///
    /// attribute.set_value("\t 456 \n");
    /// assert_eq!("456", attribute.value());
    /// # }
    /// ```
    pub fn set_value(&mut self, value: impl Into<String>) -> String {
        let mut value = value.into();
        value.trim_in_place();
        std::mem::replace(&mut self.value.0, value)
    }

    /// The attribute [`value`](Self::value) in the form of mutable [`Properties`].
    ///
    /// # Examples
    /// - Modifying an attribute value as a list of properties:
    /// ```
    /// # use rbook::ebook::element::Attribute;
    /// let mut attribute = Attribute::new("class", "title main section-1");
    ///
    /// let properties = attribute.as_properties_mut();
    /// properties.insert("new title");
    /// properties.insert("section-1");
    /// properties.remove("main");
    ///
    /// assert_eq!("title section-1 new", attribute.value());
    /// ```
    pub fn as_properties_mut(&mut self) -> &mut Properties {
        &mut self.value
    }
}

impl<N: Into<String>, V: Into<String>> From<(N, V)> for Attribute {
    fn from((name, value): (N, V)) -> Self {
        Self::new(name.into(), value.into())
    }
}

impl Attributes {
    pub(crate) fn iter_key_value(&self) -> impl Iterator<Item = (&str, &str)> {
        self.iter().map(|attr| (attr.name().as_str(), attr.value()))
    }

    /// Inserts the given attribute and returns the previous attribute with the same name, if any.
    ///
    /// # Examples
    /// - Inserting a custom attribute into a detached spine entry:
    /// ```
    /// # use rbook::epub::spine::DetachedEpubSpineEntry;
    /// let mut detached = DetachedEpubSpineEntry::new("c1");
    /// let mut entry_mut = detached.as_mut();
    /// let attributes = entry_mut.attributes_mut();
    ///
    /// // Insert a custom vendor attribute
    /// attributes.insert(("this:appearance", "omit"));
    ///
    /// assert_eq!(Some("omit"), attributes.get_value("this:appearance"));
    /// ```
    pub fn insert(&mut self, attribute: impl Into<Attribute>) -> Option<Attribute> {
        self.0.insert(attribute.into())
    }

    /// Returns the mutable [`Attribute`] with the given `name` if present, otherwise [`None`].
    pub fn by_name_mut(&mut self, name: &str) -> Option<&mut Attribute> {
        self.0.by_key_mut(name)
    }

    /// Returns an iterator over **all** mutable [`Attribute`] entries.
    pub fn iter_mut(&mut self) -> AttributesIterMut<'_> {
        AttributesIterMut(self.0.0.iter_mut())
    }

    /// Removes and returns the attribute with the given name, if present.
    pub fn remove(&mut self, name: &str) -> Option<Attribute> {
        self.0.remove(name)
    }

    /// Retains only the attributes specified by the predicate.
    ///
    /// If the closure returns `false`, the attribute is retained.
    /// Otherwise, the attribute is removed.
    ///
    /// This method operates in place and visits every attribute exactly once.
    ///
    /// # See Also
    /// - [`Self::extract_if`] to retrieve an iterator of the removed attributes.
    pub fn retain(&mut self, f: impl FnMut(&Attribute) -> bool) {
        self.0.retain(f)
    }

    /// Removes and returns only the attributes specified by the predicate.
    ///
    /// If the closure returns `true`, the attribute is removed and yielded.
    /// Otherwise, the attribute is retained.
    ///
    /// # Drop
    /// If the returned iterator is not exhausted,
    /// (e.g. dropped without iterating or iteration short-circuits),
    /// then the remaining attributes are retained.
    ///
    /// Prefer [`Self::retain`] with a negated predicate if the returned iterator is not needed.
    pub fn extract_if(
        &mut self,
        f: impl FnMut(&Attribute) -> bool,
    ) -> impl Iterator<Item = Attribute> {
        self.0.extract_if(f)
    }

    /// Removes and returns all attributes.
    pub fn drain(&mut self) -> impl Iterator<Item = Attribute> {
        self.0.drain()
    }

    /// Removes all attributes.
    ///
    /// # See Also
    /// - [`Self::drain`] to retrieve an iterator of the removed attributes.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl Extend<Attribute> for Attributes {
    fn extend<I: IntoIterator<Item = Attribute>>(&mut self, iter: I) {
        for attr in iter {
            self.insert(attr);
        }
    }
}

impl<'a> IntoIterator for &'a mut Attributes {
    type Item = &'a mut Attribute;
    type IntoIter = AttributesIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An iterator over all mutable [`Attribute`] entries within [`Attributes`].
///
/// # See Also
/// - [`Attributes::iter_mut`] to create an instance of this struct.
pub struct AttributesIterMut<'a>(std::slice::IterMut<'a, Attribute>);

impl<'a> Iterator for AttributesIterMut<'a> {
    // AttributeData is not returned directly here
    // to allow greater flexibility in the future.
    type Item = &'a mut Attribute;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
