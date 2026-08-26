use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process;

use clap::{Parser, ValueEnum};
use fracturedjson::{
    CommentPolicy, EolStyle, Formatter, FracturedJsonOptions, NumberListAlignment,
};
use is_terminal::IsTerminal;

/// A human-friendly JSON formatter with smart line breaks and table alignment.
///
/// fjson reads JSON from stdin or files and outputs beautifully formatted JSON.
/// Similar to jq but focused on producing highly readable output with aligned
/// columns and smart wrapping.
#[derive(Parser, Debug)]
#[command(name = "fjson")]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file(s). If not specified, reads from stdin.
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,

    /// Output file. If not specified, writes to stdout.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Colorize output for the terminal (stdout only).
    #[arg(long, value_enum, default_value = "auto")]
    color: ColorModeArg,

    /// Minify output (remove all whitespace).
    #[arg(short, long)]
    compact: bool,

    /// Maximum line length before wrapping.
    #[arg(short = 'w', long, default_value = "120")]
    max_width: usize,

    /// Number of spaces per indentation level.
    #[arg(short, long, default_value = "4")]
    indent: usize,

    /// Use tabs instead of spaces for indentation.
    #[arg(short = 't', long)]
    tabs: bool,

    /// Line ending style.
    #[arg(long, value_enum, default_value = "lf")]
    eol: EolStyleArg,

    /// How to handle comments in input.
    #[arg(long, value_enum, default_value = "error")]
    comments: CommentPolicyArg,

    /// Allow trailing commas in input.
    #[arg(long)]
    trailing_commas: bool,

    /// Preserve blank lines from input.
    #[arg(long)]
    preserve_blanks: bool,

    /// Number alignment style in arrays.
    #[arg(long, value_enum, default_value = "decimal")]
    number_align: NumberAlignArg,

    /// Maximum nesting depth for inline formatting (-1 to disable).
    #[arg(long, default_value = "2", allow_negative_numbers = true)]
    max_inline_complexity: isize,

    /// Maximum nesting depth for table formatting (-1 to disable).
    #[arg(long, default_value = "2", allow_negative_numbers = true)]
    max_table_complexity: isize,

    /// Add padding inside brackets for simple arrays/objects.
    #[arg(long)]
    simple_bracket_padding: bool,

    /// Disable padding inside brackets for nested arrays/objects.
    #[arg(long)]
    no_nested_bracket_padding: bool,

    /// Treat input as JSON Lines (one JSON value per line).
    #[arg(long)]
    jsonl: bool,

    /// How to handle JSONL parsing errors (only used with --jsonl).
    #[arg(long, value_enum, default_value = "fail")]
    jsonl_errors: JsonlErrorPolicy,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum EolStyleArg {
    Lf,
    Crlf,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CommentPolicyArg {
    Error,
    Remove,
    Preserve,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum NumberAlignArg {
    Left,
    Right,
    Decimal,
    Normalize,
}

/// How to handle errors when parsing JSONL input.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum JsonlErrorPolicy {
    /// Stop processing on the first error (default).
    #[default]
    Fail,
    /// Skip invalid lines and continue processing.
    Skip,
    /// Output invalid lines unchanged.
    Passthrough,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorModeArg {
    Auto,
    Always,
    Never,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("fjson: {}", e);
        process::exit(1);
    }
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    // Read input
    let input = if args.files.is_empty() {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        let mut combined = String::new();
        for path in &args.files {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("cannot read '{}': {}", path.display(), e))?;
            combined.push_str(&content);
        }
        combined
    };

    // Configure formatter
    let mut formatter = Formatter::new();
    configure_options(&mut formatter.options, &args);

    // Format
    let output = if args.jsonl {
        process_jsonl(&input, &mut formatter, args.compact, args.jsonl_errors)?
    } else if args.compact {
        formatter.minify(&input)?
    } else {
        formatter.reformat(&input, 0)?
    };

    let output = if args.output.is_none() && should_colorize(args.color) {
        colorize_json(&output)
    } else {
        output
    };

    // Write output
    if let Some(path) = args.output {
        fs::write(&path, &output)
            .map_err(|e| format!("cannot write '{}': {}", path.display(), e))?;
    } else {
        io::stdout().write_all(output.as_bytes())?;
    }

    Ok(())
}

/// Process JSONL input (one JSON value per line).
fn process_jsonl(
    input: &str,
    formatter: &mut Formatter,
    compact: bool,
    error_policy: JsonlErrorPolicy,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output_lines = Vec::new();

    for (line_num, line) in input.lines().enumerate() {
        // Preserve empty lines
        if line.trim().is_empty() {
            output_lines.push(String::new());
            continue;
        }

        // Try to format the line
        let result = if compact {
            formatter.minify(line)
        } else {
            formatter.reformat(line, 0)
        };

        match result {
            Ok(formatted) => {
                // Remove trailing newline from formatted output since we add our own
                let formatted = formatted.trim_end().to_string();
                output_lines.push(formatted);
            }
            Err(e) => match error_policy {
                JsonlErrorPolicy::Fail => {
                    return Err(format!("line {}: {}", line_num + 1, e).into());
                }
                JsonlErrorPolicy::Skip => {
                    // Skip this line entirely
                    continue;
                }
                JsonlErrorPolicy::Passthrough => {
                    // Output the original line unchanged
                    output_lines.push(line.to_string());
                }
            },
        }
    }

    // Join with newlines and add trailing newline
    let mut result = output_lines.join("\n");
    if !result.is_empty() {
        result.push('\n');
    }
    Ok(result)
}

const COLOR_RESET: &str = "\x1b[0m";
const COLOR_KEY: &str = "\x1b[94m";
const COLOR_STRING: &str = "\x1b[32m";
const COLOR_NUMBER: &str = "\x1b[36m";
const COLOR_LITERAL: &str = "\x1b[35m";
const COLOR_PUNCT: &str = "\x1b[2m";
const COLOR_COMMENT: &str = "\x1b[90m";

fn should_colorize(mode: ColorModeArg) -> bool {
    match mode {
        ColorModeArg::Auto => io::stdout().is_terminal(),
        ColorModeArg::Always => true,
        ColorModeArg::Never => false,
    }
}

enum ContainerState {
    Object(bool),
    Array,
}

fn colorize_json(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut index = 0;
    let mut containers: Vec<ContainerState> = Vec::new();

    while index < bytes.len() {
        let byte = bytes[index];

        if !byte.is_ascii() {
            let ch = input[index..].chars().next().unwrap();
            output.push(ch);
            index += ch.len_utf8();
            continue;
        }

        match byte {
            b'"' => {
                let start = index;
                index += 1;
                let mut escaped = false;
                while index < bytes.len() {
                    let current = bytes[index];
                    if current == b'\n' {
                        index += 1;
                        break;
                    }
                    if current == b'\\' && !escaped {
                        escaped = true;
                        index += 1;
                        continue;
                    }
                    if current == b'"' && !escaped {
                        index += 1;
                        break;
                    }
                    if escaped {
                        escaped = false;
                    }
                    advance_utf8(input, bytes, &mut index);
                }

                let color = if matches!(containers.last(), Some(ContainerState::Object(true))) {
                    COLOR_KEY
                } else {
                    COLOR_STRING
                };
                push_colored(&mut output, color, input, start, index);
            }
            b'/' if matches_literal(bytes, index, b"//") => {
                let start = index;
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    advance_utf8(input, bytes, &mut index);
                }
                push_colored(&mut output, COLOR_COMMENT, input, start, index);
            }
            b'/' if matches_literal(bytes, index, b"/*") => {
                let start = index;
                index += 2;
                while index + 1 < bytes.len() {
                    if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                        index += 2;
                        break;
                    }
                    advance_utf8(input, bytes, &mut index);
                }
                if index < bytes.len() && index + 1 >= bytes.len() {
                    index = bytes.len();
                }
                push_colored(&mut output, COLOR_COMMENT, input, start, index);
            }
            b'-' | b'0'..=b'9' => {
                if byte == b'-' && (index + 1 >= bytes.len() || !bytes[index + 1].is_ascii_digit())
                {
                    output.push('-');
                    index += 1;
                    continue;
                }
                let start = index;
                index += 1;
                while index < bytes.len() {
                    let current = bytes[index];
                    if current.is_ascii_digit()
                        || matches!(current, b'.' | b'e' | b'E' | b'+' | b'-')
                    {
                        index += 1;
                    } else {
                        break;
                    }
                }
                push_colored(&mut output, COLOR_NUMBER, input, start, index);
            }
            b't' if matches_literal(bytes, index, b"true") => {
                let start = index;
                index += 4;
                push_colored(&mut output, COLOR_LITERAL, input, start, index);
            }
            b'f' if matches_literal(bytes, index, b"false") => {
                let start = index;
                index += 5;
                push_colored(&mut output, COLOR_LITERAL, input, start, index);
            }
            b'n' if matches_literal(bytes, index, b"null") => {
                let start = index;
                index += 4;
                push_colored(&mut output, COLOR_LITERAL, input, start, index);
            }
            b'{' => {
                containers.push(ContainerState::Object(true));
                let start = index;
                index += 1;
                push_colored(&mut output, COLOR_PUNCT, input, start, index);
            }
            b'}' => {
                if let Some(ContainerState::Object(_)) = containers.last() {
                    containers.pop();
                }
                let start = index;
                index += 1;
                push_colored(&mut output, COLOR_PUNCT, input, start, index);
            }
            b'[' => {
                containers.push(ContainerState::Array);
                let start = index;
                index += 1;
                push_colored(&mut output, COLOR_PUNCT, input, start, index);
            }
            b']' => {
                if let Some(ContainerState::Array) = containers.last() {
                    containers.pop();
                }
                let start = index;
                index += 1;
                push_colored(&mut output, COLOR_PUNCT, input, start, index);
            }
            b':' => {
                if let Some(ContainerState::Object(expect_key)) = containers.last_mut() {
                    *expect_key = false;
                }
                let start = index;
                index += 1;
                push_colored(&mut output, COLOR_PUNCT, input, start, index);
            }
            b',' => {
                if let Some(ContainerState::Object(expect_key)) = containers.last_mut() {
                    *expect_key = true;
                }
                let start = index;
                index += 1;
                push_colored(&mut output, COLOR_PUNCT, input, start, index);
            }
            _ => {
                output.push(byte as char);
                index += 1;
            }
        }
    }

    output
}

fn matches_literal(bytes: &[u8], index: usize, literal: &[u8]) -> bool {
    bytes.len() >= index + literal.len() && &bytes[index..index + literal.len()] == literal
}

fn push_colored(output: &mut String, color: &str, input: &str, start: usize, end: usize) {
    output.push_str(color);
    output.push_str(&input[start..end]);
    output.push_str(COLOR_RESET);
}

fn advance_utf8(input: &str, bytes: &[u8], index: &mut usize) {
    if bytes[*index].is_ascii() {
        *index += 1;
    } else if let Some(ch) = input[*index..].chars().next() {
        *index += ch.len_utf8();
    } else {
        *index += 1;
    }
}

fn configure_options(opts: &mut FracturedJsonOptions, args: &Args) {
    opts.max_total_line_length = args.max_width;
    opts.indent_spaces = args.indent;
    opts.use_tab_to_indent = args.tabs;

    opts.json_eol_style = match args.eol {
        EolStyleArg::Lf => EolStyle::Lf,
        EolStyleArg::Crlf => EolStyle::Crlf,
    };

    opts.comment_policy = match args.comments {
        CommentPolicyArg::Error => CommentPolicy::TreatAsError,
        CommentPolicyArg::Remove => CommentPolicy::Remove,
        CommentPolicyArg::Preserve => CommentPolicy::Preserve,
    };

    opts.number_list_alignment = match args.number_align {
        NumberAlignArg::Left => NumberListAlignment::Left,
        NumberAlignArg::Right => NumberListAlignment::Right,
        NumberAlignArg::Decimal => NumberListAlignment::Decimal,
        NumberAlignArg::Normalize => NumberListAlignment::Normalize,
    };

    opts.allow_trailing_commas = args.trailing_commas;
    opts.preserve_blank_lines = args.preserve_blanks;
    opts.max_inline_complexity = args.max_inline_complexity;
    opts.max_table_row_complexity = args.max_table_complexity;
    opts.simple_bracket_padding = args.simple_bracket_padding;
    opts.nested_bracket_padding = !args.no_nested_bracket_padding;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorize_json_highlights_tokens() {
        let input = r#"{"key":true,"num":-3.5,"text":"hi","nil":null,/*c*/"arr":[1]}"#;
        let output = colorize_json(input);

        assert!(output.contains(&format!("{COLOR_KEY}\"key\"{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_STRING}\"hi\"{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_NUMBER}-3.5{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_NUMBER}1{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_LITERAL}true{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_LITERAL}null{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_COMMENT}/*c*/{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_KEY}\"arr\"{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_PUNCT}{{{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_PUNCT}}}{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_PUNCT}[{COLOR_RESET}")));
        assert!(output.contains(&format!("{COLOR_PUNCT}]{COLOR_RESET}")));
    }
}
