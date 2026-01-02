use fracturedjson::{CommentPolicy, EolStyle, Formatter, FracturedJsonOptions, NumberListAlignment, TableCommaPlacement};
use std::fs;
use std::path::Path;

#[derive(Clone)]
struct UniversalTestParams {
    text: String,
    opts: FracturedJsonOptions,
}

#[test]
fn universal_is_well_formed() {
    for params in generate_universal_params() {
        let mut formatter = Formatter::new();
        formatter.options = params.opts.clone();

        if formatter.options.comment_policy == CommentPolicy::Preserve {
            formatter.options.comment_policy = CommentPolicy::Remove;
        }

        let output_text = formatter.reformat(&params.text, 0).unwrap();
        serde_json::from_str::<serde_json::Value>(&output_text).unwrap();
    }
}

#[test]
fn universal_all_strings_exist() {
    for params in generate_universal_params() {
        let mut formatter = Formatter::new();
        formatter.options = params.opts.clone();
        let output_text = formatter.reformat(&params.text, 0).unwrap();

        let mut start_pos = 0usize;
        let chars: Vec<char> = params.text.chars().collect();
        while start_pos < chars.len() {
            while start_pos < chars.len() && chars[start_pos] != '"' {
                start_pos += 1;
            }
            let mut end_pos = start_pos + 1;
            while end_pos < chars.len() && chars[end_pos] != '"' {
                end_pos += 1;
            }
            if end_pos >= chars.len() {
                break;
            }
            let string_from_source: String = chars[start_pos + 1..end_pos].iter().collect();
            assert!(output_text.contains(&string_from_source));
            start_pos = end_pos + 1;
        }
    }
}

#[test]
fn universal_max_length_respected() {
    for params in generate_universal_params() {
        let mut formatter = Formatter::new();
        formatter.options = params.opts.clone();
        let output_text = formatter.reformat(&params.text, 0).unwrap();
        let output_lines: Vec<&str> = output_text.trim_end().split(eol_string(&params.opts)).collect();

        for line in output_lines {
            if line.chars().count() <= params.opts.max_total_line_length {
                continue;
            }
            let comma_count = line.matches(',').count();
            assert!(comma_count <= 1);
        }
    }
}

#[test]
fn universal_max_inline_complexity_respected() {
    for params in generate_universal_params() {
        let mut formatter = Formatter::new();
        formatter.options = params.opts.clone();
        let output_text = formatter.reformat(&params.text, 0).unwrap();
        let output_lines: Vec<&str> = output_text.trim_end().split(eol_string(&params.opts)).collect();

        let biggest_complexity = std::cmp::max(
            0,
            std::cmp::max(
                params.opts.max_inline_complexity,
                std::cmp::max(params.opts.max_compact_array_complexity, params.opts.max_table_row_complexity),
            ),
        );

        for line in output_lines {
            let mut open_count: isize = 0;
            let mut nest_level: isize = 0;
            let mut top_level_comma_seen = false;
            let mut multiple_top_level_items = false;
            for ch in line.chars() {
                match ch {
                    ' ' | '\t' => {}
                    '[' | '{' => {
                        if top_level_comma_seen && open_count == 0 {
                            multiple_top_level_items = true;
                        }
                        open_count += 1;
                    }
                    ']' | '}' => {
                        open_count -= 1;
                        nest_level = nest_level.max(open_count);
                    }
                    _ => {
                        if top_level_comma_seen && open_count == 0 {
                            multiple_top_level_items = true;
                        }
                        if ch == ',' && open_count == 0 {
                            top_level_comma_seen = true;
                        }
                        nest_level = nest_level.max(open_count);
                    }
                }
            }

            if multiple_top_level_items && params.opts.comment_policy != CommentPolicy::Preserve {
                assert!(nest_level <= params.opts.max_compact_array_complexity);
                continue;
            }

            assert!(nest_level <= biggest_complexity);
        }
    }
}

#[test]
fn universal_repeated_formatting_is_stable() {
    for params in generate_universal_params() {
        let mut main_formatter = Formatter::new();
        main_formatter.options = params.opts.clone();
        let initial_output = main_formatter.reformat(&params.text, 0).unwrap();

        let crunch_output = main_formatter.minify(&initial_output).unwrap();
        let back_to_start_output1 = main_formatter.reformat(&crunch_output, 0).unwrap();
        assert_eq!(back_to_start_output1, initial_output);

        let mut expand_options = FracturedJsonOptions::default();
        expand_options.always_expand_depth = isize::MAX;
        expand_options.comment_policy = CommentPolicy::Preserve;
        expand_options.preserve_blank_lines = true;
        expand_options.number_list_alignment = NumberListAlignment::Decimal;

        let mut expand_formatter = Formatter::new();
        expand_formatter.options = expand_options;

        let expand_output = expand_formatter.reformat(&crunch_output, 0).unwrap();
        let back_to_start_output2 = main_formatter.reformat(&expand_output, 0).unwrap();
        assert_eq!(back_to_start_output2, initial_output);
    }
}

#[test]
fn universal_no_trailing_whitespace() {
    for params in generate_universal_params() {
        let mut formatter = Formatter::new();
        formatter.options = params.opts.clone();
        let output_text = formatter.reformat(&params.text, 0).unwrap();
        let output_lines: Vec<&str> = output_text.trim_end().split(eol_string(&params.opts)).collect();

        for line in output_lines {
            let trimmed = line.trim_end();
            assert_eq!(line, trimmed);
        }
    }
}

fn generate_universal_params() -> Vec<UniversalTestParams> {
    let standard_base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/StandardJsonFiles");
    let mut standard_file_list: Vec<_> = fs::read_dir(standard_base_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();
    standard_file_list.sort();

    let standard_content_list: Vec<String> = standard_file_list
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect();
    let standard_options_list = generate_options();

    let mut params = Vec::new();
    for file_contents in &standard_content_list {
        for option in &standard_options_list {
            params.push(UniversalTestParams {
                text: file_contents.clone(),
                opts: option.clone(),
            });
        }
    }

    let comments_base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/FilesWithComments");
    let mut comments_file_list: Vec<_> = fs::read_dir(comments_base_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();
    comments_file_list.sort();

    let comments_content_list: Vec<String> = comments_file_list
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect();
    let mut comments_options_list = generate_options();
    for opts in &mut comments_options_list {
        opts.comment_policy = CommentPolicy::Preserve;
        opts.preserve_blank_lines = true;
    }

    for file_contents in &comments_content_list {
        for option in &comments_options_list {
            params.push(UniversalTestParams {
                text: file_contents.clone(),
                opts: option.clone(),
            });
        }
    }

    params
}

fn generate_options() -> Vec<FracturedJsonOptions> {
    let mut opts_list = Vec::new();

    for inline in -1..=3 {
        for array in -1..=3 {
            for table in -1..=3 {
                let mut opts = FracturedJsonOptions::default();
                opts.max_inline_complexity = inline;
                opts.max_compact_array_complexity = array;
                opts.max_table_row_complexity = table;
                opts_list.push(opts);
            }
        }
    }

    for len in 12..=55 {
        let mut opts = FracturedJsonOptions::default();
        opts.max_total_line_length = len;
        opts_list.push(opts);
    }

    let mut opts = FracturedJsonOptions::default();
    opts_list.push(opts.clone());

    opts = FracturedJsonOptions::default();
    opts.max_inline_complexity = 10000;
    opts_list.push(opts.clone());

    opts = FracturedJsonOptions::default();
    opts.json_eol_style = EolStyle::Crlf;
    opts_list.push(opts.clone());

    opts = FracturedJsonOptions::default();
    opts.json_eol_style = EolStyle::Lf;
    opts_list.push(opts.clone());

    opts = FracturedJsonOptions::default();
    opts.max_inline_complexity = 10;
    opts.max_compact_array_complexity = 10;
    opts.max_table_row_complexity = 10;
    opts.max_total_line_length = 1000;
    opts_list.push(opts.clone());

    opts = FracturedJsonOptions::default();
    opts.nested_bracket_padding = false;
    opts.simple_bracket_padding = true;
    opts.colon_padding = false;
    opts.comment_padding = false;
    opts.indent_spaces = 3;
    opts.prefix_string = "\t\t".to_string();
    opts_list.push(opts.clone());

    opts = FracturedJsonOptions::default();
    opts.table_comma_placement = TableCommaPlacement::BeforePadding;
    opts.number_list_alignment = NumberListAlignment::Left;
    opts_list.push(opts.clone());

    opts = FracturedJsonOptions::default();
    opts.table_comma_placement = TableCommaPlacement::BeforePaddingExceptNumbers;
    opts.number_list_alignment = NumberListAlignment::Decimal;
    opts_list.push(opts.clone());

    opts = FracturedJsonOptions::default();
    opts.table_comma_placement = TableCommaPlacement::BeforePaddingExceptNumbers;
    opts.number_list_alignment = NumberListAlignment::Normalize;
    opts_list.push(opts.clone());

    opts_list.push(FracturedJsonOptions::recommended());

    opts_list
}

fn eol_string(options: &FracturedJsonOptions) -> &'static str {
    match options.json_eol_style {
        EolStyle::Crlf => "\r\n",
        EolStyle::Lf => "\n",
    }
}
