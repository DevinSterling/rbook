// Utility functions module
use super::{Attribute, Element};
use lol_html::html_content::Attribute as LolAttribute;

pub(crate) fn copy_attributes(old_attributes: &[LolAttribute]) -> Vec<Attribute> {
    old_attributes
        .iter()
        .map(|attr| Attribute {
            name: attr.name(),
            value: attr.value(),
        })
        .collect()
}

pub(crate) fn equals_attribute_by_value(element: &Element, field: &str, value: &str) -> bool {
    element.get_attribute(field).map_or(false, |attribute| {
        attribute.split_whitespace().any(|slice| slice == value)
    })
}
