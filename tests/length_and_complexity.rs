mod helpers;

use fracturedjson::Formatter;
use helpers::normalize_quotes;

#[test]
fn correct_line_count_for_inline_complexity() {
    let cases = vec![(4, 1), (3, 3), (2, 6), (1, 9), (0, 14)];
    for (max_comp, exp_lines) in cases {
        let input_lines = [
            "[",
            "    { 'Q': [ {'foo': 'bar'}, 678 ], 'R': [ {}, 'asdf'] }",
            "]",
        ];
        let input = normalize_quotes(&input_lines.join("\n"));
        let mut formatter = Formatter::new();
        formatter.options.max_total_line_length = 90;
        formatter.options.max_inline_complexity = max_comp;
        formatter.options.max_compact_array_complexity = -1;
        formatter.options.max_table_row_complexity = -1;

        let output = formatter.reformat(&input, 0).unwrap();
        let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
        assert_eq!(output_lines.len(), exp_lines, "max_comp={}", max_comp);
    }
}

#[test]
fn correct_line_count_for_multiline_compact() {
    let cases = vec![(2, 5), (1, 9)];
    for (max_comp, exp_lines) in cases {
        let input_lines = [
            "[",
            "    [1,2,3], [4,5,6], [7,8,9], [null,11,12], [13,14,15], [16,17,18], [19,null,21]",
            "]",
        ];
        let input = normalize_quotes(&input_lines.join("\n"));
        let mut formatter = Formatter::new();
        formatter.options.max_total_line_length = 60;
        formatter.options.max_inline_complexity = 2;
        formatter.options.max_compact_array_complexity = max_comp;
        formatter.options.max_table_row_complexity = -1;

        let output = formatter.reformat(&input, 0).unwrap();
        let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
        assert_eq!(output_lines.len(), exp_lines, "max_comp={}", max_comp);
    }
}

#[test]
fn correct_line_count_for_line_length() {
    let cases = vec![(100, 3, 1), (90, 3, 4), (70, 3, 5), (50, 3, 9), (57, 3, 9), (50, 2, 6)];
    for (total_len, items_per_row, exp_lines) in cases {
        let input_lines = [
            "[",
            "    [1,2,3], [4,5,6], [7,8,9], [null,11,12], [13,14,15], [16,17,18], [19,null,21]",
            "]",
        ];
        let input = normalize_quotes(&input_lines.join("\n"));
        let mut formatter = Formatter::new();
        formatter.options.max_total_line_length = total_len;
        formatter.options.max_inline_complexity = 2;
        formatter.options.max_compact_array_complexity = 2;
        formatter.options.max_table_row_complexity = 2;
        formatter.options.min_compact_array_row_items = items_per_row;

        let output = formatter.reformat(&input, 0).unwrap();
        let output_lines: Vec<&str> = output.trim_end().split('\n').collect();
        assert_eq!(output_lines.len(), exp_lines, "total_len={}, items_per_row={}", total_len, items_per_row);
    }
}
