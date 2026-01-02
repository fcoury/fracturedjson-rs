use fracturedjson::{Formatter, NumberListAlignment};
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::path::Path;

#[test]
fn matches_native_stringify_when_minimized() {
    let simple_cases: Vec<serde_json::Value> = vec![
        serde_json::Value::Null,
        json!("shoehorn with teeth"),
        json!(18),
        json!([]),
        json!({}),
        json!(true),
        json!(""),
        json!({ "a": "foo", "b": false, "c": 0 }),
        json!([[1, 2, null], [4, null, 6], {"x":7, "y":8, "z":9}]),
    ];

    for element in simple_cases {
        let native_minified = serde_json::to_string(&element).unwrap();
        let mut formatter = Formatter::new();
        formatter.options.number_list_alignment = NumberListAlignment::Left;
        let nicely_formatted = formatter.serialize_value(&element, 0, 100).unwrap();

        let fj_minified = formatter.minify(&nicely_formatted).unwrap();
        assert_eq!(fj_minified, native_minified);
    }
}

#[test]
fn throws_if_recursion_limit_exceeded() {
    let mut value = json!([]);
    for _ in 0..10 {
        value = json!([value]);
    }

    let mut formatter = Formatter::new();
    let result = formatter.serialize_value(&value, 0, 5);
    assert!(result.is_err());
}

#[test]
fn handles_sparse_arrays() {
    #[derive(Serialize)]
    struct Sparse<'a>(Vec<Option<&'a str>>);

    let arr = Sparse(vec![Some("val1"), None, None, Some("val2")]);

    let mut formatter = Formatter::new();
    let nice = formatter.serialize(&arr.0, 0, 100).unwrap();
    assert_eq!(nice, "[\"val1\", null, null, \"val2\"]\n");
}

#[test]
fn file_data_matches_native_stringify_when_minimized() {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/StandardJsonFiles");
    let mut files: Vec<_> = fs::read_dir(base_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();
    files.sort();

    for path in files {
        let file_data = fs::read_to_string(&path).unwrap();
        let element: serde_json::Value = serde_json::from_str(&file_data).unwrap();
        let native_minified = serde_json::to_string(&element).unwrap();

        let mut formatter = Formatter::new();
        formatter.options.number_list_alignment = NumberListAlignment::Left;
        formatter.options.max_table_row_complexity = -1;

        let nicely_formatted = formatter.serialize_value(&element, 0, 100).unwrap();
        let fj_minified = formatter.minify(&nicely_formatted).unwrap();
        assert_eq!(fj_minified, native_minified);
    }
}
