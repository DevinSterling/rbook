use lol_html::html_content::Attribute as LolAttribute;

use crate::formats::xml::{Attribute, Element};

const WILDCARD: &str = "*";

pub(crate) fn copy_attributes(old_attributes: &[LolAttribute]) -> Vec<Attribute> {
    old_attributes
        .iter()
        .map(|attribute| Attribute::new(attribute.name(), attribute.value()))
        .collect()
}

pub(crate) fn equals_attribute_by_value(element: &Element, field: &str, value: &str) -> bool {
    element.get_attribute(field).map_or(false, |attribute| {
        attribute.split_whitespace().any(|slice| slice == value)
    })
}

pub(crate) fn find_helper<'a, F>(mut input: &str, fallback: F) -> Option<Vec<&'a Element>>
where
    F: Fn(&str, bool) -> Option<Vec<&'a Element>>,
{
    // Remove namespace
    if let Some((_, right)) = crate::utility::split_where(input, ':') {
        input = right
    }

    let mut result: Option<Vec<&Element>> = None;

    for field in input.split('>') {
        let split_field = crate::utility::split_exclude(field, &['[', ']']);
        let (element, attributes) = split_field.split_at(1);
        let element_name = element[0].trim();

        // Whether to check field names of elements
        let is_wildcard = element_name == WILDCARD;
        let elements = match result.take() {
            // Find using existing elements
            Some(results) => Some(if is_wildcard {
                results
                    .into_iter()
                    .filter_map(|element| element.children())
                    .flatten()
                    .collect()
            } else {
                results
                    .into_iter()
                    .filter_map(|element| element.get_child(element_name))
                    .collect()
            }),
            None => fallback(element_name, is_wildcard),
        }?;

        result.replace(find_elements(&elements, element_name, attributes)?);
    }

    result
}

pub(crate) fn find_elements<'a>(
    elements: &[&'a Element],
    name: &str,
    attributes: &[&str],
) -> Option<Vec<&'a Element>> {
    let (equals_attribute, has_attribute) = find_prepare(attributes);

    let result: Vec<_> = elements
        .iter()
        .filter(|&element| filter_element(element, name, &has_attribute, &equals_attribute))
        .copied()
        .collect();

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn find_prepare<'a>(attributes: &[&'a str]) -> (Vec<(&'a str, &'a str)>, Vec<&'a str>) {
    let (equals_attribute, has_attribute): (Vec<&str>, Vec<&str>) = attributes
        .iter()
        .partition(|attribute| attribute.contains('='));

    // Now make equals_attribute hold tuples instead.
    // Where each entry is (attribute field, attribute value)
    let equals_attribute: Vec<(&str, &str)> = equals_attribute
        .into_iter()
        .map(|attribute| crate::utility::split_where(attribute, '=').expect("Should contain `=`"))
        .collect();

    (equals_attribute, has_attribute)
}

fn filter_element(
    element: &Element,
    name: &str,
    has_attribute: &[&str],
    equals_attribute: &[(&str, &str)],
) -> bool {
    (name == WILDCARD || element.name() == name)
        && has_attribute
            .iter()
            .all(|attribute| element.contains_attribute(attribute.trim()))
        && equals_attribute
            .iter()
            .all(|(attribute_name, attribute_value)| {
                equals_attribute_by_value(element, attribute_name, attribute_value)
            })
}
