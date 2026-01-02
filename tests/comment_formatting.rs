mod helpers;

use fracturedjson::{CommentPolicy, Formatter};
use helpers::{do_instances_line_up, normalize_quotes};

#[test]
fn pre_and_post_comments_stay_with_elems() {
    let input_lines = [
        "{",
        "    /*1*/ 'a': [true, true], /*2*/",
        "    'b': [false, false], ",
        "    /*3*/ 'c': [false, true] /*4*/",
        "}",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    formatter.options.max_inline_complexity = 2;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
    assert_eq!(output_lines.len(), 1);

    formatter.options.max_inline_complexity = 1;
    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
    assert_eq!(output_lines.len(), 5);
    assert!(output_lines[1].contains("\"a\""));
    assert!(output_lines[1].contains("/*2*/"));
    assert!(output_lines[3].contains("\"c\""));
    assert!(output_lines[3].contains("/*3*/"));

    formatter.options.max_inline_complexity = 0;
    formatter.options.max_compact_array_complexity = 0;
    formatter.options.max_table_row_complexity = 0;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 14);
    assert!(output_lines[1].contains("/*1*/ \"a\""));
    assert!(output_lines[4].contains("] /*2*/,"));
    assert!(output_lines[9].contains("/*3*/ \"c\""));
    assert!(output_lines[12].contains("] /*4*/"));
}

#[test]
fn blank_lines_force_expanded() {
    let input_lines = ["    [ 1,", "    ", "    2 ]"];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
    assert_eq!(output_lines.len(), 1);

    formatter.options.preserve_blank_lines = true;
    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
    assert_eq!(output_lines.len(), 5);
}

#[test]
fn can_inline_middle_comments_if_no_line_break() {
    let input_lines = ["{'a': /*1*/", "[true,true]}"];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
    assert_eq!(output_lines.len(), 1);
    assert!(output_lines[0].contains("/*1*/"));

    formatter.options.max_inline_complexity = 0;
    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
    assert!(output_lines[1].contains("\"a\": /*1*/ ["));
}

#[test]
fn split_when_middle_comment_requires_break_1() {
    let input_lines = ["{'a': //1", "[true,true]}"];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 8);
    assert_eq!(output_lines[1].find("\"a\""), Some(4));
    assert_eq!(output_lines[2].find("//1"), Some(8));
    assert_eq!(output_lines[3].find("["), Some(8));
}

#[test]
fn split_when_middle_comment_requires_break_2() {
    let input_lines = ["{'a': /*1", "2*/ [true,true]}"];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 9);
    assert_eq!(output_lines[1].find("\"a\""), Some(4));
    assert_eq!(output_lines[2].find("/*1"), Some(8));
    assert_eq!(output_lines[4].find("["), Some(8));
}

#[test]
fn multiline_comments_preserve_relative_spacing() {
    let input_lines = [
        "[ 1,",
        "  /* +",
        "     +",
        "     + */",
        "  2]",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 7);
    assert!(do_instances_line_up(&output_lines, "+"));
}

#[test]
fn ambiguous_comments_in_arrays_respect_commas() {
    let input_lines = [
        "[ [ 'a' /*1*/, 'b' ],",
        "  [ 'c', /*2*/ 'd' ] ]",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    formatter.options.always_expand_depth = 99;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 10);
    assert!(output.contains("\"a\" /*1*/,"));
    assert!(output.contains("/*2*/ \"d\""));
}

#[test]
fn ambiguous_comments_in_objects_respect_commas() {
    let input_lines = [
        "[ { 'a':'a' /*1*/, 'b':'b' },",
        "  { 'c':'c', /*2*/ 'd':'d'} ]",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    formatter.options.always_expand_depth = 99;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 10);
    assert!(output.contains("\"a\" /*1*/,"));
    assert!(output.contains("/*2*/ \"d\""));
}

#[test]
fn top_level_comments_ignored_if_set() {
    let input_lines = ["//a", "[1,2, //b", "3]", "//c"];
    let input = normalize_quotes(&input_lines.join("\n"));
    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Remove;
    formatter.options.always_expand_depth = 99;

    let output = formatter.reformat(&input, 0).unwrap();
    assert!(!output.contains("//"));
}
