mod helpers;

use fracturedjson::{CommentPolicy, Formatter};
use helpers::normalize_quotes;

#[test]
fn no_commas_for_comments_expanded() {
    let input_lines = ["[", "/*a*/", "1, false", "/*b*/", "]"];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 6);
    let comma_count = output.matches(',').count();
    assert_eq!(comma_count, 1);
}

#[test]
fn no_commas_for_comments_table() {
    let input_lines = ["[", "/*a*/", "[1], [false]", "/*b*/", "]"];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 6);
    assert!(output.contains("[1    ]"));

    let comma_count = output.matches(',').count();
    assert_eq!(comma_count, 1);
}
