mod helpers;

use fracturedjson::Formatter;
use helpers::{do_instances_line_up, normalize_quotes};

#[test]
fn always_expand_depth_honored() {
    let input_lines = [
        "[",
        "[ {'x':1}, false ],",
        "{ 'a':[2], 'b':[3] }",
        "]",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.max_inline_complexity = 100;
    formatter.options.max_total_line_length = usize::MAX;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
    assert_eq!(output_lines.len(), 1);

    formatter.options.always_expand_depth = 0;
    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
    assert_eq!(output_lines.len(), 4);

    formatter.options.always_expand_depth = 1;
    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
    assert_eq!(output_lines.len(), 10);
}

#[test]
fn always_expand_depth_doesnt_prevent_table_formatting() {
    let input = "[ [1, 22, 9 ], [333, 4, 9 ] ]";

    let mut formatter = Formatter::new();
    formatter.options.always_expand_depth = 0;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 4);
    assert!(do_instances_line_up(&output_lines, ","));
    assert!(do_instances_line_up(&output_lines, "9"));
}
