mod helpers;

use fracturedjson::{EolStyle, Formatter, FracturedJsonOptions, NumberListAlignment, TableCommaPlacement};

#[test]
fn inline_array_doesnt_justify_numbers() {
    let input = "[1, 2.1, 3, -99]";
    let expected_output = "[1, 2.1, 3, -99]";

    let mut formatter = Formatter::new();
    let output = formatter.reformat(input, 0).unwrap();
    assert_eq!(output.trim_end(), expected_output);
}

#[test]
fn compact_array_does_justify_numbers() {
    let input = "[1, 2.1, 3, -99]";
    let expected_output = "[\n      1.0,   2.1,   3.0, -99.0\n]";

    let mut formatter = Formatter::new();
    formatter.options.max_inline_complexity = -1;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Normalize;

    let output = formatter.reformat(input, 0).unwrap();
    assert_eq!(output.trim_end(), expected_output);
}

#[test]
fn table_array_does_justify_numbers() {
    let input = "[[1, 2.1, 3, -99],[5, 6, 7, 8]]";
    let expected_output = "[\n    [1, 2.1, 3, -99],\n    [5, 6.0, 7,   8]\n]";

    let mut formatter = Formatter::new();
    formatter.options.max_inline_complexity = -1;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Normalize;
    formatter.options.table_comma_placement = TableCommaPlacement::AfterPadding;

    let output = formatter.reformat(input, 0).unwrap();
    assert_eq!(output.trim_end(), expected_output);
}

#[test]
fn big_numbers_invalidate_alignment_1() {
    let input = "[1, 2.1, 3, 1e+99]";
    let expected_output = "[\n    1    , 2.1  , 3    , 1e+99\n]";

    let mut formatter = Formatter::new();
    formatter.options.max_inline_complexity = -1;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Normalize;
    formatter.options.table_comma_placement = TableCommaPlacement::AfterPadding;

    let output = formatter.reformat(input, 0).unwrap();
    assert_eq!(output.trim_end(), expected_output);
}

#[test]
fn big_numbers_invalidate_alignment_2() {
    let input = "[1, 2.1, 3, 12345678901234567]";
    let expected_output = "[\n    1                , 2.1              , 3                , 12345678901234567\n]";

    let mut formatter = Formatter::new();
    formatter.options.max_inline_complexity = -1;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Normalize;
    formatter.options.table_comma_placement = TableCommaPlacement::AfterPadding;

    let output = formatter.reformat(input, 0).unwrap();
    assert_eq!(output.trim_end(), expected_output);
}

#[test]
fn nulls_respected_when_aligning_numbers() {
    let input = "[1, 2, null, -99]";
    let expected_output = "[\n       1,    2, null,  -99\n]";

    let mut formatter = Formatter::new();
    formatter.options.max_inline_complexity = -1;
    let output = formatter.reformat(input, 0).unwrap();

    assert_eq!(output.trim_end(), expected_output);
}

#[test]
fn overflow_double_invalidates_alignment() {
    let input = "[1e500, 4.0]";
    let expected_output = "[\n    1e500,\n    4.0\n]";

    let mut formatter = Formatter::new();
    formatter.options.max_inline_complexity = -1;
    let output = formatter.reformat(input, 0).unwrap();

    assert_eq!(output.trim_end(), expected_output);
}

#[test]
fn underflow_double_invalidates_alignment() {
    let input = "[1e-500, 4.0]";
    let expected_output = "[\n    1e-500,\n    4.0\n]";

    let mut formatter = Formatter::new();
    formatter.options.max_inline_complexity = -1;
    let output = formatter.reformat(input, 0).unwrap();

    assert_eq!(output.trim_end(), expected_output);
}

#[test]
fn accurate_composite_length_for_normalized() {
    let input_rows = [
        "[",
        "    { \"a\": {\"val\": 12345} },",
        "    { \"a\": {\"val\": 6.78901} },",
        "    { \"a\": null },",
        "    { \"a\": {\"val\": 1e500} }",
        "]",
    ];

    let input = input_rows.join("");
    let mut opts = FracturedJsonOptions::default();
    opts.max_total_line_length = 40;
    opts.json_eol_style = EolStyle::Lf;
    opts.number_list_alignment = NumberListAlignment::Normalize;

    let mut formatter = Formatter::new();
    formatter.options = opts;
    let output = formatter.reformat(&input, 0).unwrap();
    let output_rows: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_rows.len(), 6);
    assert_eq!(output_rows[2].len(), output_rows[3].len());
}

#[test]
fn left_align_matches_expected() {
    let expected_rows = [
        "[",
        "    [123.456 , 0      , 0   ],",
        "    [234567.8, 0      , 0   ],",
        "    [3       , 0.00000, 7e2 ],",
        "    [null    , 2e-1   , 80e1],",
        "    [5.6789  , 3.5e-1 , 0   ]",
        "]",
    ];
    test_alignment(NumberListAlignment::Left, &expected_rows);
}

#[test]
fn right_align_matches_expected() {
    let expected_rows = [
        "[",
        "    [ 123.456,       0,    0],",
        "    [234567.8,       0,    0],",
        "    [       3, 0.00000,  7e2],",
        "    [    null,    2e-1, 80e1],",
        "    [  5.6789,  3.5e-1,    0]",
        "]",
    ];
    test_alignment(NumberListAlignment::Right, &expected_rows);
}

#[test]
fn decimal_align_matches_expected() {
    let expected_rows = [
        "[",
        "    [   123.456 , 0      ,  0  ],",
        "    [234567.8   , 0      ,  0  ],",
        "    [     3     , 0.00000,  7e2],",
        "    [  null     , 2e-1   , 80e1],",
        "    [     5.6789, 3.5e-1 ,  0  ]",
        "]",
    ];
    test_alignment(NumberListAlignment::Decimal, &expected_rows);
}

#[test]
fn normalize_align_matches_expected() {
    let expected_rows = [
        "[",
        "    [   123.4560, 0.00,   0],",
        "    [234567.8000, 0.00,   0],",
        "    [     3.0000, 0.00, 700],",
        "    [  null     , 0.20, 800],",
        "    [     5.6789, 0.35,   0]",
        "]",
    ];
    test_alignment(NumberListAlignment::Normalize, &expected_rows);
}

fn test_alignment(align: NumberListAlignment, expected_rows: &[&str]) {
    let input_rows = [
        "[",
        "    [ 123.456, 0, 0 ],",
        "    [ 234567.8, 0, 0 ],",
        "    [ 3, 0.00000, 7e2 ],",
        "    [ null, 2e-1, 80e1 ],",
        "    [ 5.6789, 3.5e-1, 0 ]",
        "]",
    ];
    let input = input_rows.join("");

    let mut formatter = Formatter::new();
    formatter.options.max_total_line_length = 60;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = align;
    formatter.options.table_comma_placement = TableCommaPlacement::AfterPadding;

    let output = formatter.reformat(&input, 0).unwrap();
    let output_rows: Vec<&str> = output.trim_end().split('\n').collect();

    assert_eq!(output_rows, expected_rows);
}
