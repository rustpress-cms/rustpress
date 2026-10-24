//! `_fields`: keeps only the requested members of the response.

use serde_json::Value;

/// `rest_is_field_included( $field, $fields )`.
fn included(field: &str, fields: &[String]) -> bool {
    fields.iter().any(|f| f == field || f.starts_with(&format!("{field}.")) || field.starts_with(&format!("{f}.")))
}

fn filter_value(value: &mut Value, prefix: &str, fields: &[String]) {
    let Value::Object(map) = value else { return };
    map.retain(|key, _| included(&format!("{prefix}{key}"), fields));
    for (key, child) in map.iter_mut() {
        let path = format!("{prefix}{key}");
        if fields.iter().any(|f| f == &path) {
            continue;
        }
        filter_value(child, &format!("{path}."), fields);
    }
}

/// Filters one item or each item of a collection.
pub fn filter(data: &mut Value, fields: &[String]) {
    match data {
        Value::Array(items) => {
            for item in items {
                filter_value(item, "", fields);
            }
        }
        other => filter_value(other, "", fields),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn keeps_requested_members_only() {
        let mut data = json!({"id": 1, "title": {"rendered": "x", "raw": "y"}, "link": "l", "_links": {}});
        filter(&mut data, &["id".into(), "title.rendered".into()]);
        assert_eq!(data, json!({"id": 1, "title": {"rendered": "x"}}));
    }

    #[test]
    fn a_whole_object_is_kept_when_named() {
        let mut data = json!([{"id": 1, "title": {"rendered": "x"}, "a": 2}]);
        filter(&mut data, &["title".into()]);
        assert_eq!(data, json!([{"title": {"rendered": "x"}}]));
    }
}
