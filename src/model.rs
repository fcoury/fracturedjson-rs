#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonItemType {
    Null,
    False,
    True,
    String,
    Number,
    Object,
    Array,
    BlankLine,
    LineComment,
    BlockComment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Invalid,
    BeginArray,
    EndArray,
    BeginObject,
    EndObject,
    String,
    Number,
    Null,
    True,
    False,
    BlockComment,
    LineComment,
    BlankLine,
    Comma,
    Colon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BracketPaddingType {
    Empty = 0,
    Simple = 1,
    Complex = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableColumnType {
    Unknown,
    Simple,
    Number,
    Array,
    Object,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputPosition {
    pub index: usize,
    pub row: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonToken {
    pub token_type: TokenType,
    pub text: String,
    pub input_position: InputPosition,
}

#[derive(Debug, Clone)]
pub struct JsonItem {
    pub item_type: JsonItemType,
    pub input_position: InputPosition,
    pub complexity: usize,
    pub name: String,
    pub value: String,
    pub prefix_comment: String,
    pub middle_comment: String,
    pub middle_comment_has_new_line: bool,
    pub postfix_comment: String,
    pub is_post_comment_line_style: bool,
    pub name_length: usize,
    pub value_length: usize,
    pub prefix_comment_length: usize,
    pub middle_comment_length: usize,
    pub postfix_comment_length: usize,
    pub minimum_total_length: usize,
    pub requires_multiple_lines: bool,
    pub children: Vec<JsonItem>,
}

impl Default for JsonItem {
    fn default() -> Self {
        Self {
            item_type: JsonItemType::Null,
            input_position: InputPosition { index: 0, row: 0, column: 0 },
            complexity: 0,
            name: String::new(),
            value: String::new(),
            prefix_comment: String::new(),
            middle_comment: String::new(),
            middle_comment_has_new_line: false,
            postfix_comment: String::new(),
            is_post_comment_line_style: false,
            name_length: 0,
            value_length: 0,
            prefix_comment_length: 0,
            middle_comment_length: 0,
            postfix_comment_length: 0,
            minimum_total_length: 0,
            requires_multiple_lines: false,
            children: Vec::new(),
        }
    }
}
