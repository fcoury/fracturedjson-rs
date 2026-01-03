use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process;

use clap::{Parser, ValueEnum};
use fracturedjson::{
    CommentPolicy, EolStyle, Formatter, FracturedJsonOptions, NumberListAlignment,
};

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
    #[arg(long, default_value = "2")]
    max_inline_complexity: isize,

    /// Maximum nesting depth for table formatting (-1 to disable).
    #[arg(long, default_value = "2")]
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
