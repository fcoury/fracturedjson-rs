mod helpers;

use fracturedjson::Formatter;
use helpers::{do_instances_line_up, find_char_index, normalize_quotes};
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

#[test]
fn pads_wide_chars_correctly() {
    let input_lines = [
        "[",
        "    {'Name': '李小龍', 'Job': 'Actor', 'Born': 1940},",
        "    {'Name': 'Mark Twain', 'Job': 'Writer', 'Born': 1835},",
        "    {'Name': '孫子', 'Job': 'General', 'Born': -544}",
        "]",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert!(do_instances_line_up(&output_lines, "Job"));
    assert!(do_instances_line_up(&output_lines, "Born"));

    formatter.string_length_func = Arc::new(|s: &str| UnicodeWidthStr::width(s));
    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(find_char_index(output_lines[1], "Job"), Some(25));
    assert_eq!(find_char_index(output_lines[2], "Job"), Some(28));
    assert_eq!(find_char_index(output_lines[3], "Job"), Some(26));
}
