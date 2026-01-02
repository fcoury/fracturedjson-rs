mod helpers;

use fracturedjson::{CommentPolicy, Formatter};
use helpers::do_instances_line_up;

#[test]
fn prop_values_aligned() {
    let input = r#"
            {
                "num": 14,
                "string": "testing property alignment",
                "arrayWithLongName": [null, null, null]
            }
        "#;

    let mut formatter = Formatter::new();
    formatter.options.max_prop_name_padding = 15;
    formatter.options.colon_before_prop_name_padding = false;
    formatter.options.max_inline_complexity = -1;
    formatter.options.max_compact_array_complexity = -1;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 9);
    assert!(do_instances_line_up(&output_lines, ":"));
}

#[test]
fn prop_values_aligned_but_not_colons() {
    let input = r#"
            {
                "num": 14,
                "string": "testing property alignment",
                "arrayWithLongName": [null, null, null]
            }
        "#;

    let mut formatter = Formatter::new();
    formatter.options.max_prop_name_padding = 15;
    formatter.options.colon_before_prop_name_padding = true;
    formatter.options.max_inline_complexity = -1;
    formatter.options.max_compact_array_complexity = -1;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 9);
    assert!(output_lines[1].contains("\"num\":"));
    assert!(output_lines[2].contains("\"string\":"));
    assert!(output_lines[3].contains("\"arrayWithLongName\":"));
    assert_eq!(output_lines[1].find("14"), output_lines[2].find("\"testing"));
    assert_eq!(output_lines[1].find("14"), output_lines[3].find('['));
}

#[test]
fn dont_align_prop_vals_when_too_much_padding_required() {
    let input = r#"
            {
                "num": 14,
                "string": "testing property alignment",
                "arrayWithLongName": [null, null, null]
            }
        "#;

    let mut formatter = Formatter::new();
    formatter.options.max_prop_name_padding = 12;
    formatter.options.colon_before_prop_name_padding = false;
    formatter.options.max_inline_complexity = -1;
    formatter.options.max_compact_array_complexity = -1;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 9);
    assert!(output_lines[1].contains("\"num\": 14"));
    assert!(output_lines[2].contains("\"string\": \"testing"));
    assert!(output_lines[3].contains("\"arrayWithLongName\": ["));
}

#[test]
fn dont_align_prop_vals_when_multiline_comment() {
    let input = r#"
            {
                "foo": // this is foo
                    [1, 2, 4],
                "bar": null,
                "bazzzz": /* this is baz */ [0]
            }
        "#;

    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    formatter.options.colon_before_prop_name_padding = false;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 11);
    assert_ne!(output_lines[9].find(':'), output_lines[8].find(':'));
}

#[test]
fn align_prop_vals_when_simple_comment() {
    let input = r#"
            {
                "foo": /* this is foo */
                    [1, 2, 4],
                "bar": null,
                "bazzzz": /* this is baz */ [0]
            }
        "#;

    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    formatter.options.colon_before_prop_name_padding = false;
    formatter.options.max_total_line_length = 80;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 5);
    assert!(do_instances_line_up(&output_lines, "["));
}

#[test]
fn align_prop_vals_when_array_wraps() {
    let input = r#"
            {
                "foo": /* this is foo */
                    [1, 2, 4],
                "bar": null,
                "bazzzz": /* this is baz */ [0]
            }
        "#;

    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    formatter.options.colon_before_prop_name_padding = false;
    formatter.options.max_total_line_length = 38;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 7);
    assert!(do_instances_line_up(&output_lines, "["));
    assert!(do_instances_line_up(&output_lines, ":"));
}

#[test]
fn dont_align_when_simple_value_too_long() {
    let input = r#"
            {
                "foo": /* this is foo */
                    [1, 2, 4],
                "bar": null,
                "bazzzz": /* this is baz */ [0]
            }
        "#;

    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    formatter.options.colon_before_prop_name_padding = false;
    formatter.options.max_total_line_length = 36;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 7);
    assert!(output.contains("\"bar\":"));
    assert_ne!(output_lines[1].find(':'), output_lines[5].find(':'));
}
