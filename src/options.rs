#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EolStyle {
    Crlf,
    Lf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentPolicy {
    TreatAsError,
    Remove,
    Preserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberListAlignment {
    Left,
    Right,
    Decimal,
    Normalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableCommaPlacement {
    BeforePadding,
    AfterPadding,
    BeforePaddingExceptNumbers,
}

#[derive(Debug, Clone)]
pub struct FracturedJsonOptions {
    pub json_eol_style: EolStyle,
    pub max_total_line_length: usize,
    pub max_inline_complexity: isize,
    pub max_compact_array_complexity: isize,
    pub max_table_row_complexity: isize,
    pub max_prop_name_padding: usize,
    pub colon_before_prop_name_padding: bool,
    pub table_comma_placement: TableCommaPlacement,
    pub min_compact_array_row_items: usize,
    pub always_expand_depth: isize,
    pub nested_bracket_padding: bool,
    pub simple_bracket_padding: bool,
    pub colon_padding: bool,
    pub comma_padding: bool,
    pub comment_padding: bool,
    pub number_list_alignment: NumberListAlignment,
    pub indent_spaces: usize,
    pub use_tab_to_indent: bool,
    pub prefix_string: String,
    pub comment_policy: CommentPolicy,
    pub preserve_blank_lines: bool,
    pub allow_trailing_commas: bool,
}

impl Default for FracturedJsonOptions {
    fn default() -> Self {
        Self {
            json_eol_style: EolStyle::Lf,
            max_total_line_length: 120,
            max_inline_complexity: 2,
            max_compact_array_complexity: 2,
            max_table_row_complexity: 2,
            max_prop_name_padding: 16,
            colon_before_prop_name_padding: false,
            table_comma_placement: TableCommaPlacement::BeforePaddingExceptNumbers,
            min_compact_array_row_items: 3,
            always_expand_depth: -1,
            nested_bracket_padding: true,
            simple_bracket_padding: false,
            colon_padding: true,
            comma_padding: true,
            comment_padding: true,
            number_list_alignment: NumberListAlignment::Decimal,
            indent_spaces: 4,
            use_tab_to_indent: false,
            prefix_string: String::new(),
            comment_policy: CommentPolicy::TreatAsError,
            preserve_blank_lines: false,
            allow_trailing_commas: false,
        }
    }
}

impl FracturedJsonOptions {
    pub fn recommended() -> Self {
        Self::default()
    }
}
