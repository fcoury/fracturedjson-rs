use fracturedjson::Formatter;

#[test]
fn formats_simple_jsonl() {
    let input = r#"{"a":1}
{"b":2}
{"c":3}"#;

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("\"a\": 1"));
    assert!(lines[1].contains("\"b\": 2"));
    assert!(lines[2].contains("\"c\": 3"));
}

#[test]
fn minifies_simple_jsonl() {
    let input = r#"{ "a": 1 }
{ "b": 2 }
{ "c": 3 }"#;

    let mut formatter = Formatter::new();
    let output = formatter.minify_jsonl(input).unwrap();

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], r#"{"a":1}"#);
    assert_eq!(lines[1], r#"{"b":2}"#);
    assert_eq!(lines[2], r#"{"c":3}"#);
}

#[test]
fn preserves_empty_lines() {
    let input = r#"{"a":1}

{"b":2}"#;

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    let lines: Vec<&str> = output.trim_end().lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("\"a\": 1"));
    assert!(lines[1].is_empty());
    assert!(lines[2].contains("\"b\": 2"));
}

#[test]
fn handles_single_line_jsonl() {
    let input = r#"{"a":1}"#;

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    let trimmed = output.trim();
    assert!(trimmed.contains("\"a\": 1"));
}

#[test]
fn handles_mixed_json_types() {
    let input = r#"{"obj":"value"}
[1,2,3]
"string"
42
true
null"#;

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 6);
    assert!(lines[0].contains("\"obj\""));
    assert!(lines[1].contains("["));
    assert!(lines[2].contains("\"string\""));
    assert!(lines[3].contains("42"));
    assert!(lines[4].contains("true"));
    assert!(lines[5].contains("null"));
}

#[test]
fn handles_trailing_newline_in_input() {
    let input = "{ \"a\": 1 }\n";

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    // Should still produce valid output with trailing newline
    assert!(output.ends_with('\n'));
    let trimmed = output.trim();
    assert!(trimmed.contains("\"a\": 1"));
}

#[test]
fn handles_windows_line_endings() {
    let input = "{\"a\":1}\r\n{\"b\":2}\r\n";

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"a\": 1"));
    assert!(lines[1].contains("\"b\": 2"));
}

#[test]
fn error_includes_line_number() {
    let input = r#"{"a":1}
invalid json
{"c":3}"#;

    let mut formatter = Formatter::new();
    let result = formatter.reformat_jsonl(input);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.message.contains("line 2"),
        "Error should mention line 2, got: {}",
        error.message
    );
}

#[test]
fn handles_whitespace_only_lines() {
    let input = "{\"a\":1}\n   \n{\"b\":2}";

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    let lines: Vec<&str> = output.trim_end().lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("\"a\": 1"));
    assert!(lines[1].is_empty()); // whitespace-only line becomes empty
    assert!(lines[2].contains("\"b\": 2"));
}

#[test]
fn handles_complex_objects() {
    let input = r#"{"name":"Alice","scores":[95,87,92]}
{"name":"Bob","scores":[88,90,85]}"#;

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    // Each line should be independently formatted
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2);

    // Both should contain their data
    let first_half = lines[0];
    let second_half = lines[1];
    assert!(first_half.contains("Alice"));
    assert!(second_half.contains("Bob"));
}

#[test]
fn empty_input_produces_empty_output() {
    let input = "";

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    assert!(output.is_empty());
}

#[test]
fn only_empty_lines_produces_empty_lines() {
    let input = "\n\n";

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    // Input has two newlines, which means two empty lines
    // Output should have them preserved
    assert!(output.contains("\n"));
}

#[test]
fn formats_arrays_inline_when_simple() {
    let input = "[1,2,3]\n[4,5,6]";

    let mut formatter = Formatter::new();
    let output = formatter.reformat_jsonl(input).unwrap();

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2);

    // Simple arrays should stay on single line
    assert!(lines[0].starts_with("[") && lines[0].ends_with("]"));
    assert!(lines[1].starts_with("[") && lines[1].ends_with("]"));
}
