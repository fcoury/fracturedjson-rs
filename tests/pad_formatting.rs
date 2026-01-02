mod helpers;

use fracturedjson::Formatter;
use std::fs;
use std::path::Path;

#[test]
fn no_spaces_anywhere() {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/StandardJsonFiles/1.json");
    let input = fs::read_to_string(base_dir).unwrap();

    let mut formatter = Formatter::new();
    formatter.options.use_tab_to_indent = true;
    formatter.options.colon_padding = false;
    formatter.options.comma_padding = false;
    formatter.options.nested_bracket_padding = false;
    formatter.options.simple_bracket_padding = false;
    formatter.options.max_compact_array_complexity = 0;
    formatter.options.max_table_row_complexity = -1;
    formatter.options.max_prop_name_padding = 0;

    let output = formatter.reformat(&input, 0).unwrap();
    assert!(!output.contains(' '));
}

#[test]
fn simple_bracket_padding_works_for_tables() {
    let input = "[[1, 2],[3, 4]]";

    let mut formatter = Formatter::new();
    formatter.options.max_inline_complexity = 1;
    formatter.options.simple_bracket_padding = true;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 4);
    assert!(output_lines[1].contains("[ 1, 2 ]"));
    assert!(output_lines[2].contains("[ 3, 4 ]"));

    formatter.options.simple_bracket_padding = false;
    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 4);
    assert!(output_lines[1].contains("[1, 2]"));
    assert!(output_lines[2].contains("[3, 4]"));
}
