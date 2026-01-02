mod helpers;

use fracturedjson::{CommentPolicy, EolStyle, Formatter, NumberListAlignment, TableCommaPlacement};
use helpers::{do_instances_line_up, normalize_quotes};

#[test]
fn nested_elements_line_up() {
    let input_lines = [
        "{",
        "    'Rect' : { 'position': {'x': -44, 'y':  3.4}, 'color': [0, 255, 255] }, ",
        "    'Point': { 'position': {'y': 22, 'z': 3} }, ",
        "    'Oval' : { 'position': {'x': 140, 'y':  0.04}, 'color': '#7f3e96' }  ",
        "}",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Normalize;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert!(do_instances_line_up(&output_lines, "x"));
    assert!(do_instances_line_up(&output_lines, "y"));
    assert!(do_instances_line_up(&output_lines, "z"));
    assert!(do_instances_line_up(&output_lines, "position"));
    assert!(do_instances_line_up(&output_lines, "color"));

    assert!(output_lines[2].contains("22.00,"));
}

#[test]
fn nested_elements_compact_when_needed_1() {
    let input_lines = [
        "{",
        "    'Rect' : { 'position': {'x': -44, 'y':  3.4}, 'color': [0, 255, 255] }, ",
        "    'Point': { 'position': {'y': 22, 'z': 3} }, ",
        "    'Oval' : { 'position': {'x': 140, 'y':  0.04}, 'color': '#7f3e96' }  ",
        "}",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.max_total_line_length = 77;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert!(do_instances_line_up(&output_lines, "position"));
    assert!(do_instances_line_up(&output_lines, "color"));
    assert!(output_lines[2].contains("22,"));
}

#[test]
fn nested_elements_compact_when_needed_2() {
    let input_lines = [
        "{",
        "    'Rect' : { 'position': {'x': -44, 'y':  3.4}, 'color': [0, 255, 255] }, ",
        "    'Point': { 'position': {'y': 22, 'z': 3} }, ",
        "    'Oval' : { 'position': {'x': 140, 'y':  0.04}, 'color': '#7f3e96' }  ",
        "}",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.max_total_line_length = 74;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.max_prop_name_padding = 0;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 5);
    assert_ne!(output_lines[1].find("position"), output_lines[2].find("position"));
}

#[test]
fn tables_with_comments_line_up() {
    let input_lines = [
        "{",
        "'Firetruck': /* red */ { 'color': '#CC0000' }, ",
        "'Dumptruck': /* yellow */ { 'color': [255, 255, 0] }, ",
        "'Godzilla': /* green */  { 'color': '#336633' },  // Not a truck",
        "/* ! */ 'F150': { 'color': null } ",
        "}",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.max_total_line_length = 100;
    formatter.options.comment_policy = CommentPolicy::Preserve;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 6);
    assert!(do_instances_line_up(&output_lines, "\""));
    assert!(do_instances_line_up(&output_lines, ":"));
    assert!(do_instances_line_up(&output_lines, " {"));
    assert!(do_instances_line_up(&output_lines, " }"));
    assert!(do_instances_line_up(&output_lines, "color"));
}

#[test]
fn tables_with_blank_lines_line_up() {
    let input_lines = ["{'a': [7,8],", "", "//1", "'b': [9,10]}"];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    formatter.options.preserve_blank_lines = true;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 6);
    assert!(do_instances_line_up(&output_lines, ":"));
    assert!(do_instances_line_up(&output_lines, "["));
    assert!(do_instances_line_up(&output_lines, "]"));
}

#[test]
fn reject_objects_with_duplicate_keys() {
    let input_lines = [
        "[ { 'x': 1, 'y': 2, 'z': 3 },",
        "{ 'y': 44, 'z': 55, 'z': 66 } ]",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.max_inline_complexity = 1;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 4);
    assert_ne!(output_lines[1].find('y'), output_lines[2].find('y'));
    let z_count = output.matches('z').count();
    assert_eq!(z_count, 3);
}

#[test]
fn commas_before_padding_works() {
    let input_lines = [
        "{",
        "    'Rect' : { 'glow': 'steady', 'position': {'x': -44, 'y':  4}, 'color': [0, 255, 255] }, ",
        "    'Point': { 'glow': 'pulse', 'position': {'y': 22, 'z': 3} }, ",
        "    'Oval' : { 'glow': 'gradient', 'position': {'x': 140.33, 'y':  0.1}, 'color': '#7f3e96' }  ",
        "}",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.max_total_line_length = 120;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Decimal;
    formatter.options.table_comma_placement = TableCommaPlacement::BeforePadding;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 5);
    assert!(output_lines[1].contains("\"steady\","));
    assert!(output_lines[2].contains("\"pulse\","));
    assert!(output_lines[3].contains("\"gradient\","));
    assert!(output_lines[1].contains("-44,"));
    assert!(output_lines[2].contains("22,"));
}

#[test]
fn commas_after_padding_works() {
    let input_lines = [
        "{",
        "    'Rect' : { 'glow': 'steady', 'position': {'x': -44, 'y':  4}, 'color': [0, 255, 255] }, ",
        "    'Point': { 'glow': 'pulse', 'position': {'y': 22, 'z': 3} }, ",
        "    'Oval' : { 'glow': 'gradient', 'position': {'x': 140.33, 'y':  0.1}, 'color': '#7f3e96' }  ",
        "}",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.max_total_line_length = 120;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Decimal;
    formatter.options.table_comma_placement = TableCommaPlacement::AfterPadding;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 5);
    assert!(output_lines[1].contains("\"steady\" "));
    assert!(output_lines[2].contains("\"pulse\" "));
    assert!(output_lines[3].contains("\"gradient\","));
    assert!(output_lines[1].contains("-44 "));
    assert!(output_lines[2].contains("22 "));
    assert!(output_lines[3].contains("140.33,"));
    assert!(do_instances_line_up(&output_lines, ","));
}

#[test]
fn commas_before_padding_except_numbers_works() {
    let input_lines = [
        "{",
        "    'Rect' : { 'glow': 'steady', 'position': {'x': -44, 'y':  4}, 'color': [0, 255, 255] }, ",
        "    'Point': { 'glow': 'pulse', 'position': {'y': 22, 'z': 3} }, ",
        "    'Oval' : { 'glow': 'gradient', 'position': {'x': 140.33, 'y':  0.1}, 'color': '#7f3e96' }  ",
        "}",
    ];
    let input = normalize_quotes(&input_lines.join("\n"));

    let mut formatter = Formatter::new();
    formatter.options.max_total_line_length = 120;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Decimal;
    formatter.options.table_comma_placement = TableCommaPlacement::BeforePaddingExceptNumbers;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 5);
    assert!(output_lines[1].contains("\"steady\","));
    assert!(output_lines[2].contains("\"pulse\","));
    assert!(output_lines[3].contains("\"gradient\","));
    assert!(output_lines[1].contains("-44 "));
    assert!(output_lines[2].contains("22 "));
    assert!(output_lines[3].contains("140.33,"));
    assert!(do_instances_line_up(&output_lines, ", \"y\":"));
}

#[test]
fn commas_before_padding_works_with_comments() {
    let input = r#"
            [
                [ 1 /* q */, "a" ], /* w */
                [ 22, "bbb" ], // x
                [ 3.33 /* sss */, "cc" ] /* y */
            ]
        "#;

    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    formatter.options.max_total_line_length = 40;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Decimal;
    formatter.options.table_comma_placement = TableCommaPlacement::BeforePadding;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_lines.len(), 5);
    assert!(output_lines[1].contains("*/,"));
    assert!(output_lines[2].contains("22,"));
    assert!(output_lines[3].contains("*/,"));

    assert_eq!(output_lines[1].find("],"), output_lines[2].find("],"));
    assert_eq!(output_lines[1].find("/* w"), output_lines[2].find("// x"));
    assert_eq!(output_lines[2].find("// x"), output_lines[3].find("/* y"));
}

#[test]
fn commas_after_padding_works_with_comments() {
    let input = r#"
            [
                [ 1 /* q */, "a" ], /* w */
                [ 22, "bbb" ], // x
                [ 3.33 /* sss */, "cc" ] /* y */
            ]
        "#;

    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    formatter.options.max_total_line_length = 40;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Decimal;
    formatter.options.table_comma_placement = TableCommaPlacement::AfterPadding;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert!(do_instances_line_up(&output_lines, ","));

    assert_eq!(output_lines[1].find("],"), output_lines[2].find("],"));
    assert_eq!(output_lines[1].find("/* w"), output_lines[2].find("// x"));
    assert_eq!(output_lines[2].find("// x"), output_lines[3].find("/* y"));
}

#[test]
fn handles_nulls_with_array_table_columns() {
    let input = r#"
            [
                {"Thing": null /* q */}, 
                {"Thing": [5] /* r */} 
            ]
        "#;

    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert!(do_instances_line_up(&output_lines, "}"));
    assert!(do_instances_line_up(&output_lines, "*/"));
}

#[test]
fn colons_hug_prop_names() {
    let input = r#"
            {
                "twos": [2, 4, 6, 8],
                "threes": [3, 6, 9, 12],
                "fours": [4, 8, 12, 16]
            }
        "#;

    let mut formatter = Formatter::new();
    formatter.options.max_total_line_length = 40;
    formatter.options.colon_before_prop_name_padding = true;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 5);
    assert!(do_instances_line_up(&output_lines, "["));
    assert!(do_instances_line_up(&output_lines, "]"));
    assert!(output_lines[1].contains("\":"));
    assert!(output_lines[2].contains("\":"));
    assert!(output_lines[3].contains("\":"));
}

#[test]
fn single_columns_with_eol_comments_work() {
    let input = r#"
            {
                "Beatles Songs": [
                    "Taxman"        ,  // George
                    "Hey Jude"      ,  // Paul
                    "Act Naturally" ,  // Ringo
                    "Ticket To Ride"   // John
                ]
            }
        "#;

    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 8);
    assert!(do_instances_line_up(&output_lines, "//"));
}

#[test]
fn single_columns_with_numbers_work() {
    let input = r#"
            {
                "WeightsKg": {
                    "Brown Bear": 389.0,
                    "Golden Retriever": 29.0,
                    "Garter Snake": 0.25
                }
            }
        "#;

    let mut formatter = Formatter::new();
    formatter.options.max_compact_array_complexity = -1;
    formatter.options.max_inline_complexity = -1;
    formatter.options.number_list_alignment = NumberListAlignment::Decimal;
    formatter.options.max_total_line_length = 40;

    let output = formatter.reformat(input, 0).unwrap();
    let output_lines: Vec<String> = output.trim_end().split('\n').map(|s| s.to_string()).collect();

    assert_eq!(output_lines.len(), 7);
    assert!(do_instances_line_up(&output_lines, "."));
}
