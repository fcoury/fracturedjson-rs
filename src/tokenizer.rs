use crate::error::FracturedJsonError;
use crate::model::{InputPosition, JsonToken, TokenType};

const MAX_DOC_SIZE: usize = 2_000_000_000;

#[derive(Clone)]
pub struct ScannerState {
    original_text: String,
    chars: Vec<char>,
    byte_indices: Vec<usize>,
    pub current_position: InputPosition,
    pub token_position: InputPosition,
    pub non_whitespace_since_last_newline: bool,
}

impl ScannerState {
    pub fn new(original_text: &str) -> Self {
        let mut chars: Vec<char> = Vec::new();
        let mut byte_indices: Vec<usize> = Vec::new();
        for (idx, ch) in original_text.char_indices() {
            byte_indices.push(idx);
            chars.push(ch);
        }
        byte_indices.push(original_text.len());

        Self {
            original_text: original_text.to_string(),
            chars,
            byte_indices,
            current_position: InputPosition { index: 0, row: 0, column: 0 },
            token_position: InputPosition { index: 0, row: 0, column: 0 },
            non_whitespace_since_last_newline: false,
        }
    }

    pub fn advance(&mut self, is_whitespace: bool) {
        if self.current_position.index >= MAX_DOC_SIZE {
            panic!("Maximum document length exceeded");
        }
        self.current_position.index += 1;
        self.current_position.column += 1;
        if !is_whitespace {
            self.non_whitespace_since_last_newline = true;
        }
    }

    pub fn new_line(&mut self) {
        if self.current_position.index >= MAX_DOC_SIZE {
            panic!("Maximum document length exceeded");
        }
        self.current_position.index += 1;
        self.current_position.row += 1;
        self.current_position.column = 0;
        self.non_whitespace_since_last_newline = false;
    }

    pub fn set_token_start(&mut self) {
        self.token_position = self.current_position;
    }

    pub fn make_token_from_buffer(&self, token_type: TokenType, trim_end: bool) -> JsonToken {
        let start = self.byte_indices[self.token_position.index];
        let end = self.byte_indices[self.current_position.index];
        let mut substring = self.original_text[start..end].to_string();
        if trim_end {
            substring = substring.trim_end().to_string();
        }
        JsonToken {
            token_type,
            text: substring,
            input_position: self.token_position,
        }
    }

    pub fn make_token(&self, token_type: TokenType, text: &str) -> JsonToken {
        JsonToken {
            token_type,
            text: text.to_string(),
            input_position: self.token_position,
        }
    }

    pub fn current(&self) -> Option<char> {
        if self.at_end() {
            None
        } else {
            Some(self.chars[self.current_position.index])
        }
    }

    pub fn at_end(&self) -> bool {
        self.current_position.index >= self.chars.len()
    }

    pub fn error(&self, message: &str) -> FracturedJsonError {
        FracturedJsonError::new(message, Some(self.current_position))
    }
}

pub struct TokenGenerator {
    state: ScannerState,
}

impl TokenGenerator {
    pub fn new(input_json: &str) -> Self {
        Self { state: ScannerState::new(input_json) }
    }
}

impl Iterator for TokenGenerator {
    type Item = Result<JsonToken, FracturedJsonError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.state.at_end() {
                return None;
            }

            let ch = self.state.current()?;
            match ch {
                ' ' | '\t' | '\r' => {
                    self.state.advance(true);
                }
                '\n' => {
                    let token = if !self.state.non_whitespace_since_last_newline {
                        Some(self.state.make_token(TokenType::BlankLine, "\n"))
                    } else {
                        None
                    };
                    self.state.new_line();
                    self.state.set_token_start();
                    if let Some(token) = token {
                        return Some(Ok(token));
                    }
                }
                '{' => return Some(process_single_char(&mut self.state, "{", TokenType::BeginObject)),
                '}' => return Some(process_single_char(&mut self.state, "}", TokenType::EndObject)),
                '[' => return Some(process_single_char(&mut self.state, "[", TokenType::BeginArray)),
                ']' => return Some(process_single_char(&mut self.state, "]", TokenType::EndArray)),
                ':' => return Some(process_single_char(&mut self.state, ":", TokenType::Colon)),
                ',' => return Some(process_single_char(&mut self.state, ",", TokenType::Comma)),
                't' => return Some(process_keyword(&mut self.state, "true", TokenType::True)),
                'f' => return Some(process_keyword(&mut self.state, "false", TokenType::False)),
                'n' => return Some(process_keyword(&mut self.state, "null", TokenType::Null)),
                '/' => return Some(process_comment(&mut self.state)),
                '"' => return Some(process_string(&mut self.state)),
                '-' => return Some(process_number(&mut self.state)),
                _ => {
                    if !is_digit(ch) {
                        return Some(Err(self.state.error("Unexpected character")));
                    }
                    return Some(process_number(&mut self.state));
                }
            }
        }
    }
}

fn process_single_char(
    state: &mut ScannerState,
    symbol: &str,
    token_type: TokenType,
) -> Result<JsonToken, FracturedJsonError> {
    state.set_token_start();
    let token = state.make_token(token_type, symbol);
    state.advance(false);
    Ok(token)
}

fn process_keyword(
    state: &mut ScannerState,
    keyword: &str,
    token_type: TokenType,
) -> Result<JsonToken, FracturedJsonError> {
    state.set_token_start();
    let mut chars = keyword.chars();
    chars.next();
    for expected in chars {
        if state.at_end() {
            return Err(state.error("Unexpected end of input while processing keyword"));
        }
        state.advance(false);
        if state.at_end() {
            return Err(state.error("Unexpected end of input while processing keyword"));
        }
        let current = state.current().unwrap();
        if current != expected {
            return Err(state.error("Unexpected keyword"));
        }
    }

    let token = state.make_token(token_type, keyword);
    state.advance(false);
    Ok(token)
}

fn process_comment(state: &mut ScannerState) -> Result<JsonToken, FracturedJsonError> {
    state.set_token_start();

    if state.at_end() {
        return Err(state.error("Unexpected end of input while processing comment"));
    }

    state.advance(false);
    let mut is_block_comment = false;
    match state.current() {
        Some('*') => is_block_comment = true,
        Some('/') => {}
        _ => return Err(state.error("Bad character for start of comment")),
    }

    state.advance(false);
    let mut last_char_was_asterisk = false;
    loop {
        if state.at_end() {
            if is_block_comment {
                return Err(state.error("Unexpected end of input while processing comment"));
            }
            return Ok(state.make_token_from_buffer(TokenType::LineComment, true));
        }

        let ch = state.current().unwrap();
        if ch == '\n' {
            state.new_line();
            if !is_block_comment {
                return Ok(state.make_token_from_buffer(TokenType::LineComment, true));
            }
            continue;
        }

        state.advance(false);
        if ch == '/' && last_char_was_asterisk {
            return Ok(state.make_token_from_buffer(TokenType::BlockComment, false));
        }
        last_char_was_asterisk = ch == '*';
    }
}

fn process_string(state: &mut ScannerState) -> Result<JsonToken, FracturedJsonError> {
    state.set_token_start();
    state.advance(false);

    let mut last_char_began_escape = false;
    let mut expected_hex_count = 0usize;
    loop {
        if state.at_end() {
            return Err(state.error("Unexpected end of input while processing string"));
        }

        let ch = state.current().unwrap();

        if expected_hex_count > 0 {
            if !is_hex(ch) {
                return Err(state.error("Bad unicode escape in string"));
            }
            expected_hex_count -= 1;
            state.advance(false);
            continue;
        }

        if last_char_began_escape {
            if !is_legal_after_backslash(ch) {
                return Err(state.error("Bad escaped character in string"));
            }
            if ch == 'u' {
                expected_hex_count = 4;
            }
            last_char_began_escape = false;
            state.advance(false);
            continue;
        }

        if is_control(ch) {
            return Err(state.error("Control characters are not allowed in strings"));
        }

        state.advance(false);
        if ch == '"' {
            return Ok(state.make_token_from_buffer(TokenType::String, false));
        }
        if ch == '\\' {
            last_char_began_escape = true;
        }
    }
}

fn process_number(state: &mut ScannerState) -> Result<JsonToken, FracturedJsonError> {
    state.set_token_start();
    let mut phase = NumberPhase::Beginning;
    loop {
        if state.at_end() {
            return match phase {
                NumberPhase::PastFirstDigitOfWhole
                | NumberPhase::PastWhole
                | NumberPhase::PastFirstDigitOfFractional
                | NumberPhase::PastFirstDigitOfExponent => Ok(state.make_token_from_buffer(TokenType::Number, false)),
                _ => Err(state.error("Unexpected end of input while processing number")),
            };
        }

        let ch = state.current().unwrap();
        let mut handling = CharHandling::ValidAndConsumed;

        match phase {
            NumberPhase::Beginning => {
                if ch == '-' {
                    phase = NumberPhase::PastLeadingSign;
                } else if ch == '0' {
                    phase = NumberPhase::PastWhole;
                } else if is_digit(ch) {
                    phase = NumberPhase::PastFirstDigitOfWhole;
                } else {
                    handling = CharHandling::InvalidatesToken;
                }
            }
            NumberPhase::PastLeadingSign => {
                if !is_digit(ch) {
                    handling = CharHandling::InvalidatesToken;
                } else if ch == '0' {
                    phase = NumberPhase::PastWhole;
                } else {
                    phase = NumberPhase::PastFirstDigitOfWhole;
                }
            }
            NumberPhase::PastFirstDigitOfWhole => {
                if ch == '.' {
                    phase = NumberPhase::PastDecimalPoint;
                } else if ch == 'e' || ch == 'E' {
                    phase = NumberPhase::PastE;
                } else if !is_digit(ch) {
                    handling = CharHandling::StartOfNewToken;
                }
            }
            NumberPhase::PastWhole => {
                if ch == '.' {
                    phase = NumberPhase::PastDecimalPoint;
                } else if ch == 'e' || ch == 'E' {
                    phase = NumberPhase::PastE;
                } else {
                    handling = CharHandling::StartOfNewToken;
                }
            }
            NumberPhase::PastDecimalPoint => {
                if is_digit(ch) {
                    phase = NumberPhase::PastFirstDigitOfFractional;
                } else {
                    handling = CharHandling::InvalidatesToken;
                }
            }
            NumberPhase::PastFirstDigitOfFractional => {
                if ch == 'e' || ch == 'E' {
                    phase = NumberPhase::PastE;
                } else if !is_digit(ch) {
                    handling = CharHandling::StartOfNewToken;
                }
            }
            NumberPhase::PastE => {
                if ch == '+' || ch == '-' {
                    phase = NumberPhase::PastExpSign;
                } else if is_digit(ch) {
                    phase = NumberPhase::PastFirstDigitOfExponent;
                } else {
                    handling = CharHandling::InvalidatesToken;
                }
            }
            NumberPhase::PastExpSign => {
                if is_digit(ch) {
                    phase = NumberPhase::PastFirstDigitOfExponent;
                } else {
                    handling = CharHandling::InvalidatesToken;
                }
            }
            NumberPhase::PastFirstDigitOfExponent => {
                if !is_digit(ch) {
                    handling = CharHandling::StartOfNewToken;
                }
            }
        }

        if handling == CharHandling::InvalidatesToken {
            return Err(state.error("Bad character while processing number"));
        }

        if handling == CharHandling::StartOfNewToken {
            return Ok(state.make_token_from_buffer(TokenType::Number, false));
        }

        state.advance(false);
    }
}

fn is_digit(ch: char) -> bool {
    ch >= '0' && ch <= '9'
}

fn is_hex(ch: char) -> bool {
    (ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')
}

fn is_legal_after_backslash(ch: char) -> bool {
    matches!(ch, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u')
}

fn is_control(ch: char) -> bool {
    let code = ch as u32;
    (code <= 0x1F) || (code == 0x7F) || (code >= 0x80 && code <= 0x9F)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumberPhase {
    Beginning,
    PastLeadingSign,
    PastFirstDigitOfWhole,
    PastWhole,
    PastDecimalPoint,
    PastFirstDigitOfFractional,
    PastE,
    PastExpSign,
    PastFirstDigitOfExponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharHandling {
    InvalidatesToken,
    ValidAndConsumed,
    StartOfNewToken,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{InputPosition, JsonToken, TokenType};

    #[test]
    fn echoes_tokens() {
        let cases: Vec<(&str, TokenType)> = vec![
            ("{", TokenType::BeginObject),
            ("}", TokenType::EndObject),
            ("[", TokenType::BeginArray),
            ("]", TokenType::EndArray),
            (":", TokenType::Colon),
            (",", TokenType::Comma),
            ("true", TokenType::True),
            ("false", TokenType::False),
            ("null", TokenType::Null),
            ("\"simple\"", TokenType::String),
            ("\"with \\t escapes\\u80fE\\r\\n\"", TokenType::String),
            ("\"\"", TokenType::String),
            ("3", TokenType::Number),
            ("3.0", TokenType::Number),
            ("-3", TokenType::Number),
            ("-3.0", TokenType::Number),
            ("0", TokenType::Number),
            ("-0", TokenType::Number),
            ("0.0", TokenType::Number),
            ("9000", TokenType::Number),
            ("3e2", TokenType::Number),
            ("3.01e+2", TokenType::Number),
            ("3e-2", TokenType::Number),
            ("-3.01E-2", TokenType::Number),
            ("\n", TokenType::BlankLine),
            ("//\n", TokenType::LineComment),
            ("// comment\n", TokenType::LineComment),
            ("// comment", TokenType::LineComment),
            ("/**/", TokenType::BlockComment),
            ("/* comment */", TokenType::BlockComment),
            ("/* comment\n *with* newline */", TokenType::BlockComment),
        ];

        for (input, token_type) in cases {
            let possibly_trimmed = if token_type == TokenType::LineComment {
                input.trim_end().to_string()
            } else {
                input.to_string()
            };

            let results: Vec<JsonToken> = match TokenGenerator::new(input).collect::<Result<Vec<_>, _>>() {
                Ok(tokens) => tokens,
                Err(err) => panic!("input={} err={}", input, err),
            };
            assert_eq!(results.len(), 1, "input={}", input);
            assert_eq!(results[0].text, possibly_trimmed);
            assert_eq!(results[0].token_type, token_type);
        }
    }

    #[test]
    fn correct_position_for_second_token() {
        let cases: Vec<(&str, usize, usize, usize)> = vec![
            ("{,", 1, 0, 1),
            ("null,", 4, 0, 4),
            ("3,", 1, 0, 1),
            ("3.12,", 4, 0, 4),
            ("3e2,", 3, 0, 3),
            ("\"st\",", 4, 0, 4),
            ("null ,", 5, 0, 5),
            ("null\t,", 5, 0, 5),
            ("null\n,", 5, 1, 0),
            (" null \r\n ,", 9, 1, 1),
            ("//co\n,", 5, 1, 0),
            ("/**/,", 4, 0, 4),
            ("/*1*/,", 5, 0, 5),
            ("/*1\n*/,", 6, 1, 2),
            ("\n\n", 1, 1, 0),
        ];

        for (input, index, row, column) in cases {
            let results: Vec<JsonToken> = match TokenGenerator::new(input).collect::<Result<Vec<_>, _>>() {
                Ok(tokens) => tokens,
                Err(err) => panic!("input={} err={}", input, err),
            };
            assert_eq!(results.len(), 2);
            assert_eq!(results[1].input_position.index, index);
            assert_eq!(results[1].input_position.row, row);
            assert_eq!(results[1].input_position.column, column);

            let expected_text = if results[0].token_type == TokenType::BlankLine {
                input[..index].to_string()
            } else {
                input[..index].trim().to_string()
            };
            assert_eq!(results[0].text, expected_text);
        }
    }

    #[test]
    fn throw_if_unexpected_end() {
        let cases = vec![
            "t",
            "nul",
            "/",
            "/*",
            "/* comment *",
            "\"",
            "\"string",
            "\"string with escaped quote\\\"",
            "1.",
            "-",
            "1.0e",
            "1.0e+",
        ];

        for input in cases {
            let result: Result<Vec<JsonToken>, FracturedJsonError> = TokenGenerator::new(input).collect();
            assert!(result.is_err(), "input={}", input);
            let err = result.err().unwrap();
            let pos = err.input_position.unwrap();
            assert_eq!(pos.index, input.chars().count());
        }
    }

    #[test]
    fn token_sequences_match_sample() {
        let input_rows = vec![
            "{                           ",
            "    // A line comment       ",
            "    \"item1\": \"a string\",    ",
            "                            ",
            "    /* a block              ",
            "       comment */           ",
            "    \"item2\": [null, -2.0]   ",
            "}                           ",
        ];
        let input_string = input_rows.join("\r\n");
        let block_comment_text = format!("{}\r\n{}", input_rows[4].trim_start(), input_rows[5].trim_end());

        let expected_tokens = vec![
            JsonToken { token_type: TokenType::BeginObject, text: "{".to_string(), input_position: InputPosition { index: 0, row: 0, column: 0 } },
            JsonToken { token_type: TokenType::LineComment, text: "// A line comment".to_string(), input_position: InputPosition { index: 34, row: 1, column: 4 } },
            JsonToken { token_type: TokenType::String, text: "\"item1\"".to_string(), input_position: InputPosition { index: 64, row: 2, column: 4 } },
            JsonToken { token_type: TokenType::Colon, text: ":".to_string(), input_position: InputPosition { index: 71, row: 2, column: 11 } },
            JsonToken { token_type: TokenType::String, text: "\"a string\"".to_string(), input_position: InputPosition { index: 73, row: 2, column: 13 } },
            JsonToken { token_type: TokenType::Comma, text: ",".to_string(), input_position: InputPosition { index: 83, row: 2, column: 23 } },
            JsonToken { token_type: TokenType::BlankLine, text: "\n".to_string(), input_position: InputPosition { index: 90, row: 3, column: 0 } },
            JsonToken { token_type: TokenType::BlockComment, text: block_comment_text, input_position: InputPosition { index: 124, row: 4, column: 4 } },
            JsonToken { token_type: TokenType::String, text: "\"item2\"".to_string(), input_position: InputPosition { index: 184, row: 6, column: 4 } },
            JsonToken { token_type: TokenType::Colon, text: ":".to_string(), input_position: InputPosition { index: 191, row: 6, column: 11 } },
            JsonToken { token_type: TokenType::BeginArray, text: "[".to_string(), input_position: InputPosition { index: 193, row: 6, column: 13 } },
            JsonToken { token_type: TokenType::Null, text: "null".to_string(), input_position: InputPosition { index: 194, row: 6, column: 14 } },
            JsonToken { token_type: TokenType::Comma, text: ",".to_string(), input_position: InputPosition { index: 198, row: 6, column: 18 } },
            JsonToken { token_type: TokenType::Number, text: "-2.0".to_string(), input_position: InputPosition { index: 200, row: 6, column: 20 } },
            JsonToken { token_type: TokenType::EndArray, text: "]".to_string(), input_position: InputPosition { index: 204, row: 6, column: 24 } },
            JsonToken { token_type: TokenType::EndObject, text: "}".to_string(), input_position: InputPosition { index: 210, row: 7, column: 0 } },
        ];

        let results: Vec<JsonToken> = match TokenGenerator::new(&input_string).collect::<Result<Vec<_>, _>>() {
            Ok(tokens) => tokens,
            Err(err) => panic!("err={}", err),
        };

        assert_eq!(results, expected_tokens);
    }

    #[test]
    fn empty_input_is_handled() {
        let results: Vec<JsonToken> = TokenGenerator::new("").collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(results.len(), 0);
    }
}
