use crate::error::FracturedJsonError;
use crate::model::{InputPosition, JsonItem, JsonItemType, JsonToken, TokenType};
use crate::options::{CommentPolicy, FracturedJsonOptions};
use crate::tokenizer::TokenGenerator;

pub struct TokenEnumerator<I>
where
    I: Iterator<Item = Result<JsonToken, FracturedJsonError>>,
{
    generator: I,
    current: Option<JsonToken>,
}

impl<I> TokenEnumerator<I>
where
    I: Iterator<Item = Result<JsonToken, FracturedJsonError>>,
{
    pub fn new(generator: I) -> Self {
        Self { generator, current: None }
    }

    pub fn current(&self) -> Result<&JsonToken, FracturedJsonError> {
        self.current.as_ref().ok_or_else(|| FracturedJsonError::simple("Illegal enumerator usage"))
    }

    pub fn move_next(&mut self) -> Result<bool, FracturedJsonError> {
        match self.generator.next() {
            None => {
                self.current = None;
                Ok(false)
            }
            Some(Ok(token)) => {
                self.current = Some(token);
                Ok(true)
            }
            Some(Err(err)) => Err(err),
        }
    }
}

pub struct Parser {
    pub options: FracturedJsonOptions,
}

impl Parser {
    pub fn new(options: FracturedJsonOptions) -> Self {
        Self { options }
    }

    pub fn parse_top_level(
        &self,
        input_json: &str,
        stop_after_first_elem: bool,
    ) -> Result<Vec<JsonItem>, FracturedJsonError> {
        let token_stream = TokenGenerator::new(input_json);
        let mut enumerator = TokenEnumerator::new(token_stream);
        self.parse_top_level_from_enum(&mut enumerator, stop_after_first_elem)
    }

    fn parse_top_level_from_enum<I>(
        &self,
        enumerator: &mut TokenEnumerator<I>,
        stop_after_first_elem: bool,
    ) -> Result<Vec<JsonItem>, FracturedJsonError>
    where
        I: Iterator<Item = Result<JsonToken, FracturedJsonError>>,
    {
        let mut top_level_items: Vec<JsonItem> = Vec::new();
        let mut top_level_elem_seen = false;

        loop {
            if !enumerator.move_next()? {
                return Ok(top_level_items);
            }

            let item = self.parse_item(enumerator)?;
            let is_comment = matches!(item.item_type, JsonItemType::BlockComment | JsonItemType::LineComment);
            let is_blank = item.item_type == JsonItemType::BlankLine;

            if is_blank {
                if self.options.preserve_blank_lines {
                    top_level_items.push(item);
                }
            } else if is_comment {
                match self.options.comment_policy {
                    CommentPolicy::TreatAsError => {
                        return Err(FracturedJsonError::new(
                            "Comments not allowed with current options",
                            Some(item.input_position),
                        ));
                    }
                    CommentPolicy::Preserve => top_level_items.push(item),
                    CommentPolicy::Remove => {}
                }
            } else {
                if stop_after_first_elem && top_level_elem_seen {
                    return Err(FracturedJsonError::new(
                        "Unexpected start of second top level element",
                        Some(item.input_position),
                    ));
                }
                top_level_items.push(item);
                top_level_elem_seen = true;
            }
        }
    }

    fn parse_item<I>(&self, enumerator: &mut TokenEnumerator<I>) -> Result<JsonItem, FracturedJsonError>
    where
        I: Iterator<Item = Result<JsonToken, FracturedJsonError>>,
    {
        let current = enumerator.current()?.clone();
        match current.token_type {
            TokenType::BeginArray => self.parse_array(enumerator),
            TokenType::BeginObject => self.parse_object(enumerator),
            _ => self.parse_simple(&current),
        }
    }

    fn parse_simple(&self, token: &JsonToken) -> Result<JsonItem, FracturedJsonError> {
        let mut item = JsonItem::default();
        item.item_type = Self::item_type_from_token_type(token)?;
        item.value = token.text.clone();
        item.input_position = token.input_position;
        item.complexity = 0;
        Ok(item)
    }

    fn parse_array<I>(&self, enumerator: &mut TokenEnumerator<I>) -> Result<JsonItem, FracturedJsonError>
    where
        I: Iterator<Item = Result<JsonToken, FracturedJsonError>>,
    {
        if enumerator.current()?.token_type != TokenType::BeginArray {
            return Err(FracturedJsonError::new(
                "Parser logic error",
                Some(enumerator.current()?.input_position),
            ));
        }

        let starting_input_position = enumerator.current()?.input_position;

        let mut elem_needing_post_comment_idx: Option<usize> = None;
        let mut elem_needing_post_end_row: isize = -1;

        let mut unplaced_comment: Option<JsonItem> = None;
        let mut child_list: Vec<JsonItem> = Vec::new();
        let mut comma_status = CommaStatus::EmptyCollection;
        let mut end_of_array_found = false;
        let mut this_array_complexity = 0usize;

        while !end_of_array_found {
            let token = Self::get_next_token_or_throw(enumerator, starting_input_position)?;

            let unplaced_needs_home = unplaced_comment.as_ref().map(|comment| {
                comment.input_position.row != token.input_position.row || token.token_type == TokenType::EndArray
            }).unwrap_or(false);

            if unplaced_needs_home {
                if let Some(idx) = elem_needing_post_comment_idx {
                    if let Some(elem) = child_list.get_mut(idx) {
                        elem.postfix_comment = unplaced_comment.as_ref().unwrap().value.clone();
                        elem.is_post_comment_line_style = unplaced_comment.as_ref().unwrap().item_type == JsonItemType::LineComment;
                    }
                } else {
                    child_list.push(unplaced_comment.as_ref().unwrap().clone());
                }
                unplaced_comment = None;
            }

            if elem_needing_post_comment_idx.is_some()
                && elem_needing_post_end_row != token.input_position.row as isize
            {
                elem_needing_post_comment_idx = None;
            }

            match token.token_type {
                TokenType::EndArray => {
                    if comma_status == CommaStatus::CommaSeen && !self.options.allow_trailing_commas {
                        return Err(FracturedJsonError::new(
                            "Array may not end with a comma with current options",
                            Some(token.input_position),
                        ));
                    }
                    end_of_array_found = true;
                }
                TokenType::Comma => {
                    if comma_status != CommaStatus::ElementSeen {
                        return Err(FracturedJsonError::new(
                            "Unexpected comma in array",
                            Some(token.input_position),
                        ));
                    }
                    comma_status = CommaStatus::CommaSeen;
                }
                TokenType::BlankLine => {
                    if self.options.preserve_blank_lines {
                        child_list.push(self.parse_simple(&token)?);
                    }
                }
                TokenType::BlockComment => {
                    if self.options.comment_policy == CommentPolicy::Remove {
                        continue;
                    }
                    if self.options.comment_policy == CommentPolicy::TreatAsError {
                        return Err(FracturedJsonError::new(
                            "Comments not allowed with current options",
                            Some(token.input_position),
                        ));
                    }

                    if unplaced_comment.is_some() {
                        child_list.push(unplaced_comment.take().unwrap());
                    }

                    let comment_item = self.parse_simple(&token)?;
                    if Self::is_multiline_comment(&comment_item) {
                        child_list.push(comment_item);
                        continue;
                    }

                    if let Some(idx) = elem_needing_post_comment_idx {
                        if comma_status == CommaStatus::ElementSeen {
                            if let Some(elem) = child_list.get_mut(idx) {
                                elem.postfix_comment = comment_item.value.clone();
                                elem.is_post_comment_line_style = false;
                            }
                            elem_needing_post_comment_idx = None;
                            continue;
                        }
                    }

                    unplaced_comment = Some(comment_item);
                }
                TokenType::LineComment => {
                    if self.options.comment_policy == CommentPolicy::Remove {
                        continue;
                    }
                    if self.options.comment_policy == CommentPolicy::TreatAsError {
                        return Err(FracturedJsonError::new(
                            "Comments not allowed with current options",
                            Some(token.input_position),
                        ));
                    }

                    if unplaced_comment.is_some() {
                        child_list.push(unplaced_comment.take().unwrap());
                        child_list.push(self.parse_simple(&token)?);
                        continue;
                    }

                    if let Some(idx) = elem_needing_post_comment_idx {
                        if let Some(elem) = child_list.get_mut(idx) {
                            elem.postfix_comment = token.text.clone();
                            elem.is_post_comment_line_style = true;
                        }
                        elem_needing_post_comment_idx = None;
                        continue;
                    }

                    child_list.push(self.parse_simple(&token)?);
                }
                TokenType::False
                | TokenType::True
                | TokenType::Null
                | TokenType::String
                | TokenType::Number
                | TokenType::BeginArray
                | TokenType::BeginObject => {
                    if comma_status == CommaStatus::ElementSeen {
                        return Err(FracturedJsonError::new(
                            "Comma missing while processing array",
                            Some(token.input_position),
                        ));
                    }

                    let mut element = self.parse_item(enumerator)?;
                    comma_status = CommaStatus::ElementSeen;
                    this_array_complexity = this_array_complexity.max(element.complexity + 1);

                    if let Some(unplaced) = unplaced_comment.take() {
                        element.prefix_comment = unplaced.value;
                    }

                    child_list.push(element);
                    elem_needing_post_comment_idx = Some(child_list.len() - 1);
                    elem_needing_post_end_row = enumerator.current()?.input_position.row as isize;
                }
                _ => {
                    return Err(FracturedJsonError::new(
                        "Unexpected token in array",
                        Some(token.input_position),
                    ));
                }
            }
        }

        let mut array_item = JsonItem::default();
        array_item.item_type = JsonItemType::Array;
        array_item.input_position = starting_input_position;
        array_item.complexity = this_array_complexity;
        array_item.children = child_list;
        Ok(array_item)
    }

    fn parse_object<I>(&self, enumerator: &mut TokenEnumerator<I>) -> Result<JsonItem, FracturedJsonError>
    where
        I: Iterator<Item = Result<JsonToken, FracturedJsonError>>,
    {
        if enumerator.current()?.token_type != TokenType::BeginObject {
            return Err(FracturedJsonError::new("Parser logic error", Some(enumerator.current()?.input_position)));
        }

        let starting_input_position = enumerator.current()?.input_position;
        let mut child_list: Vec<JsonItem> = Vec::new();

        let mut property_name: Option<JsonToken> = None;
        let mut property_value: Option<JsonItem> = None;
        let mut line_prop_value_ends: isize = -1;
        let mut before_prop_comments: Vec<JsonItem> = Vec::new();
        let mut mid_prop_comments: Vec<JsonToken> = Vec::new();
        let mut after_prop_comment: Option<JsonItem> = None;
        let mut after_prop_comment_was_after_comma = false;

        let mut phase = ObjectPhase::BeforePropName;
        let mut this_obj_complexity = 0usize;
        let mut end_of_object = false;
        while !end_of_object {
            let token = Self::get_next_token_or_throw(enumerator, starting_input_position)?;

            let is_new_line = line_prop_value_ends != token.input_position.row as isize;
            let is_end_of_object = token.token_type == TokenType::EndObject;
            let starting_next_prop_name = token.token_type == TokenType::String && phase == ObjectPhase::AfterComma;
            let is_excess_post_comment = after_prop_comment.is_some()
                && matches!(token.token_type, TokenType::BlockComment | TokenType::LineComment);

            let need_to_flush = property_name.is_some()
                && property_value.is_some()
                && (is_new_line || is_end_of_object || starting_next_prop_name || is_excess_post_comment);

            if need_to_flush {
                let mut comment_to_hold_for_next_elem: Option<JsonItem> = None;
                if starting_next_prop_name && after_prop_comment_was_after_comma && !is_new_line {
                    comment_to_hold_for_next_elem = after_prop_comment.take();
                }

                Self::attach_object_value_pieces(
                    &mut child_list,
                    property_name.as_ref().unwrap(),
                    property_value.as_ref().unwrap(),
                    line_prop_value_ends,
                    &mut before_prop_comments,
                    &mut mid_prop_comments,
                    after_prop_comment.take(),
                );
                this_obj_complexity = this_obj_complexity.max(property_value.as_ref().unwrap().complexity + 1);
                property_name = None;
                property_value = None;
                before_prop_comments.clear();
                mid_prop_comments.clear();
                after_prop_comment = None;

                if let Some(comment) = comment_to_hold_for_next_elem {
                    before_prop_comments.push(comment);
                }
            }

            match token.token_type {
                TokenType::BlankLine => {
                    if !self.options.preserve_blank_lines {
                        continue;
                    }
                    if matches!(phase, ObjectPhase::AfterPropName | ObjectPhase::AfterColon) {
                        continue;
                    }
                    child_list.extend(before_prop_comments.drain(..));
                    child_list.push(self.parse_simple(&token)?);
                }
                TokenType::BlockComment | TokenType::LineComment => {
                    if self.options.comment_policy == CommentPolicy::Remove {
                        continue;
                    }
                    if self.options.comment_policy == CommentPolicy::TreatAsError {
                        return Err(FracturedJsonError::new(
                            "Comments not allowed with current options",
                            Some(token.input_position),
                        ));
                    }
                    if matches!(phase, ObjectPhase::BeforePropName) || property_name.is_none() {
                        before_prop_comments.push(self.parse_simple(&token)?);
                    } else if matches!(phase, ObjectPhase::AfterPropName | ObjectPhase::AfterColon) {
                        mid_prop_comments.push(token);
                    } else {
                        after_prop_comment = Some(self.parse_simple(&token)?);
                        after_prop_comment_was_after_comma = matches!(phase, ObjectPhase::AfterComma);
                    }
                }
                TokenType::EndObject => {
                    if matches!(phase, ObjectPhase::AfterPropName | ObjectPhase::AfterColon) {
                        return Err(FracturedJsonError::new("Unexpected end of object", Some(token.input_position)));
                    }
                    end_of_object = true;
                }
                TokenType::String => {
                    if matches!(phase, ObjectPhase::BeforePropName | ObjectPhase::AfterComma) {
                        property_name = Some(token);
                        phase = ObjectPhase::AfterPropName;
                    } else if matches!(phase, ObjectPhase::AfterColon) {
                        property_value = Some(self.parse_item(enumerator)?);
                        line_prop_value_ends = enumerator.current()?.input_position.row as isize;
                        phase = ObjectPhase::AfterPropValue;
                    } else {
                        return Err(FracturedJsonError::new(
                            "Unexpected string found while processing object",
                            Some(token.input_position),
                        ));
                    }
                }
                TokenType::False
                | TokenType::True
                | TokenType::Null
                | TokenType::Number
                | TokenType::BeginArray
                | TokenType::BeginObject => {
                    if !matches!(phase, ObjectPhase::AfterColon) {
                        return Err(FracturedJsonError::new(
                            "Unexpected element while processing object",
                            Some(token.input_position),
                        ));
                    }
                    property_value = Some(self.parse_item(enumerator)?);
                    line_prop_value_ends = enumerator.current()?.input_position.row as isize;
                    phase = ObjectPhase::AfterPropValue;
                }
                TokenType::Colon => {
                    if !matches!(phase, ObjectPhase::AfterPropName) {
                        return Err(FracturedJsonError::new(
                            "Unexpected colon while processing object",
                            Some(token.input_position),
                        ));
                    }
                    phase = ObjectPhase::AfterColon;
                }
                TokenType::Comma => {
                    if !matches!(phase, ObjectPhase::AfterPropValue) {
                        return Err(FracturedJsonError::new(
                            "Unexpected comma while processing object",
                            Some(token.input_position),
                        ));
                    }
                    phase = ObjectPhase::AfterComma;
                }
                _ => {
                    return Err(FracturedJsonError::new(
                        "Unexpected token while processing object",
                        Some(token.input_position),
                    ));
                }
            }
        }

        if !self.options.allow_trailing_commas && matches!(phase, ObjectPhase::AfterComma) {
            return Err(FracturedJsonError::new(
                "Object may not end with comma with current options",
                Some(enumerator.current()?.input_position),
            ));
        }

        let mut obj_item = JsonItem::default();
        obj_item.item_type = JsonItemType::Object;
        obj_item.input_position = starting_input_position;
        obj_item.complexity = this_obj_complexity;
        obj_item.children = child_list;
        Ok(obj_item)
    }

    fn item_type_from_token_type(token: &JsonToken) -> Result<JsonItemType, FracturedJsonError> {
        match token.token_type {
            TokenType::False => Ok(JsonItemType::False),
            TokenType::True => Ok(JsonItemType::True),
            TokenType::Null => Ok(JsonItemType::Null),
            TokenType::Number => Ok(JsonItemType::Number),
            TokenType::String => Ok(JsonItemType::String),
            TokenType::BlankLine => Ok(JsonItemType::BlankLine),
            TokenType::BlockComment => Ok(JsonItemType::BlockComment),
            TokenType::LineComment => Ok(JsonItemType::LineComment),
            _ => Err(FracturedJsonError::new("Unexpected Token", Some(token.input_position))),
        }
    }

    fn get_next_token_or_throw<I>(
        enumerator: &mut TokenEnumerator<I>,
        start_position: InputPosition,
    ) -> Result<JsonToken, FracturedJsonError>
    where
        I: Iterator<Item = Result<JsonToken, FracturedJsonError>>,
    {
        if !enumerator.move_next()? {
            return Err(FracturedJsonError::new(
                "Unexpected end of input while processing array or object starting",
                Some(start_position),
            ));
        }
        Ok(enumerator.current()?.clone())
    }

    fn is_multiline_comment(item: &JsonItem) -> bool {
        item.item_type == JsonItemType::BlockComment && item.value.contains('\n')
    }

    fn attach_object_value_pieces(
        obj_item_list: &mut Vec<JsonItem>,
        name: &JsonToken,
        element: &JsonItem,
        value_ending_line: isize,
        before_comments: &mut Vec<JsonItem>,
        mid_comments: &mut Vec<JsonToken>,
        after_comment: Option<JsonItem>,
    ) {
        let mut element = element.clone();
        element.name = name.text.clone();

        if !mid_comments.is_empty() {
            let mut combined = String::new();
            for (i, comment) in mid_comments.iter().enumerate() {
                combined.push_str(&comment.text);
                if i < mid_comments.len() - 1 || comment.token_type == TokenType::LineComment {
                    combined.push('\n');
                }
            }
            element.middle_comment = combined.clone();
            element.middle_comment_has_new_line = combined.contains('\n');
        }

        if !before_comments.is_empty() {
            let last = before_comments.pop().unwrap();
            if last.item_type == JsonItemType::BlockComment
                && last.input_position.row == element.input_position.row
            {
                element.prefix_comment = last.value;
                obj_item_list.extend(before_comments.drain(..));
            } else {
                obj_item_list.extend(before_comments.drain(..));
                obj_item_list.push(last);
            }
        }

        obj_item_list.push(element.clone());

        if let Some(after) = after_comment {
            if !Self::is_multiline_comment(&after)
                && after.input_position.row as isize == value_ending_line
            {
                let mut updated = element.clone();
                updated.postfix_comment = after.value;
                updated.is_post_comment_line_style = after.item_type == JsonItemType::LineComment;
                obj_item_list.pop();
                obj_item_list.push(updated);
            } else {
                obj_item_list.push(after);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommaStatus {
    EmptyCollection,
    ElementSeen,
    CommaSeen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectPhase {
    BeforePropName,
    AfterPropName,
    AfterColon,
    AfterPropValue,
    AfterComma,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{CommentPolicy, FracturedJsonOptions};
    use crate::model::JsonItemType;

    #[test]
    fn test_simple_and_valid_array() {
        let input = r#"[4.7, true, null, "a string", {}, false, []]"#;
        let parser = Parser::new(FracturedJsonOptions::default());
        let doc_model = parser.parse_top_level(input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].item_type, JsonItemType::Array);

        let expected_child_types = vec![
            JsonItemType::Number,
            JsonItemType::True,
            JsonItemType::Null,
            JsonItemType::String,
            JsonItemType::Object,
            JsonItemType::False,
            JsonItemType::Array,
        ];
        let found_child_types: Vec<JsonItemType> =
            doc_model[0].children.iter().map(|ch| ch.item_type).collect();
        assert_eq!(expected_child_types, found_child_types);

        let expected_text = vec!["4.7", "true", "null", "\"a string\"", "", "false", ""];
        let found_text: Vec<String> = doc_model[0].children.iter().map(|ch| ch.value.clone()).collect();
        assert_eq!(expected_text, found_text);
    }

    #[test]
    fn array_with_inline_block_comments() {
        let input = "[ /*a*/ 1 /*b*/ ]";

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 1);
        assert_eq!(doc_model[0].children[0].prefix_comment, "/*a*/");
        assert_eq!(doc_model[0].children[0].postfix_comment, "/*b*/");
    }

    #[test]
    fn array_with_mixed_inline_comments() {
        let input = ["[ /*a*/ 1 //b", "]"].join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 1);
        assert_eq!(doc_model[0].children[0].prefix_comment, "/*a*/");
        assert_eq!(doc_model[0].children[0].postfix_comment, "//b");
    }

    #[test]
    fn array_with_unattached_trailing_comments() {
        let input = ["[ /*a*/ 1 /*b*/ /*c*/", "]"].join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 2);
        assert_eq!(doc_model[0].children[0].item_type, JsonItemType::Number);
        assert_eq!(doc_model[0].children[0].prefix_comment, "/*a*/");
        assert_eq!(doc_model[0].children[0].postfix_comment, "/*b*/");
        assert_eq!(doc_model[0].children[1].item_type, JsonItemType::BlockComment);
        assert_eq!(doc_model[0].children[1].value, "/*c*/");
    }

    #[test]
    fn array_with_multiple_leading_comments() {
        let input = "[ /*a*/ /*b*/ 1 ]";

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 2);
        assert_eq!(doc_model[0].children[0].item_type, JsonItemType::BlockComment);
        assert_eq!(doc_model[0].children[0].value, "/*a*/");
        assert_eq!(doc_model[0].children[1].item_type, JsonItemType::Number);
        assert_eq!(doc_model[0].children[1].prefix_comment, "/*b*/");
    }

    #[test]
    fn array_ambiguous_comment_precedes_comma() {
        let input = "[ /*a*/ 1 /*b*/, 2 /*c*/ ]";

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 2);
        assert_eq!(doc_model[0].children[0].prefix_comment, "/*a*/");
        assert_eq!(doc_model[0].children[0].postfix_comment, "/*b*/");
        assert_eq!(doc_model[0].children[1].prefix_comment_length, 0);
        assert_eq!(doc_model[0].children[1].postfix_comment, "/*c*/");
    }

    #[test]
    fn array_ambiguous_comment_follows_comma() {
        let input = "[ /*a*/ 1, /*b*/ 2 /*c*/ ]";

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 2);
        assert_eq!(doc_model[0].children[0].prefix_comment, "/*a*/");
        assert_eq!(doc_model[0].children[0].postfix_comment_length, 0);
        assert_eq!(doc_model[0].children[1].prefix_comment, "/*b*/");
        assert_eq!(doc_model[0].children[1].postfix_comment, "/*c*/");
    }

    #[test]
    fn array_ambiguous_comment_follows_comma_with_newline() {
        let input = ["[ /*a*/ 1, /*b*/", "2 /*c*/ ]"].join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 2);
        assert_eq!(doc_model[0].children[0].prefix_comment, "/*a*/");
        assert_eq!(doc_model[0].children[0].postfix_comment, "/*b*/");
        assert_eq!(doc_model[0].children[1].prefix_comment_length, 0);
        assert_eq!(doc_model[0].children[1].postfix_comment, "/*c*/");
    }

    #[test]
    fn array_multiple_unattached_comments() {
        let input = ["[", "    /*a*/ //b", "    null", "]"].join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 3);
        assert_eq!(doc_model[0].children[0].value, "/*a*/");
        assert_eq!(doc_model[0].children[1].value, "//b");
        assert_eq!(doc_model[0].children[2].item_type, JsonItemType::Null);
    }

    #[test]
    fn array_multiple_comment_stands_alone() {
        let input = ["[", "    1, /*a", "    b*/ 2", "]"].join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 3);
        assert_eq!(doc_model[0].children[0].value, "1");
        assert_eq!(doc_model[0].children[1].value, "/*a\r\n    b*/");
        assert_eq!(doc_model[0].children[2].value, "2");
    }

    #[test]
    fn array_blank_lines_are_preserved_or_removed() {
        let input = ["[", "", "    //comment", "    true,", "", "    ", "    false", "]"].join("\r\n");

        let mut preserve_options = FracturedJsonOptions::default();
        preserve_options.comment_policy = CommentPolicy::Preserve;
        preserve_options.allow_trailing_commas = true;
        preserve_options.preserve_blank_lines = true;

        let preserve_parser = Parser::new(preserve_options);
        let preserve_doc_model = preserve_parser.parse_top_level(&input, false).unwrap();

        assert_eq!(preserve_doc_model.len(), 1);
        assert_eq!(preserve_doc_model[0].item_type, JsonItemType::Array);
        let preserve_expected_types = vec![
            JsonItemType::BlankLine,
            JsonItemType::LineComment,
            JsonItemType::True,
            JsonItemType::BlankLine,
            JsonItemType::BlankLine,
            JsonItemType::False,
        ];
        let preserve_found_types: Vec<JsonItemType> =
            preserve_doc_model[0].children.iter().map(|ch| ch.item_type).collect();
        assert_eq!(preserve_expected_types, preserve_found_types);

        let mut remove_options = FracturedJsonOptions::default();
        remove_options.comment_policy = CommentPolicy::Remove;
        remove_options.allow_trailing_commas = true;
        remove_options.preserve_blank_lines = false;

        let remove_parser = Parser::new(remove_options);
        let remove_doc_model = remove_parser.parse_top_level(&input, false).unwrap();

        assert_eq!(remove_doc_model.len(), 1);
        assert_eq!(remove_doc_model[0].item_type, JsonItemType::Array);
        let remove_expected_types = vec![JsonItemType::True, JsonItemType::False];
        let remove_found_types: Vec<JsonItemType> =
            remove_doc_model[0].children.iter().map(|ch| ch.item_type).collect();
        assert_eq!(remove_expected_types, remove_found_types);
    }

    #[test]
    fn test_simple_and_valid_object() {
        let input = "{ \"a\": 5.2, \"b\": false, \"c\": null, \"d\": true, \"e\":[], \"f\":{}, \"g\": \"a string\" }";
        let parser = Parser::new(FracturedJsonOptions::default());
        let doc_model = parser.parse_top_level(input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].item_type, JsonItemType::Object);

        let expected_child_types = vec![
            JsonItemType::Number,
            JsonItemType::False,
            JsonItemType::Null,
            JsonItemType::True,
            JsonItemType::Array,
            JsonItemType::Object,
            JsonItemType::String,
        ];
        let found_child_types: Vec<JsonItemType> =
            doc_model[0].children.iter().map(|ch| ch.item_type).collect();
        assert_eq!(expected_child_types, found_child_types);

        let expected_prop_names = vec![
            "\"a\"", "\"b\"", "\"c\"", "\"d\"", "\"e\"", "\"f\"", "\"g\"",
        ];
        let found_prop_names: Vec<String> = doc_model[0].children.iter().map(|ch| ch.name.clone()).collect();
        assert_eq!(expected_prop_names, found_prop_names);

        let expected_text = vec!["5.2", "false", "null", "true", "", "", "\"a string\""];        
        let found_text: Vec<String> = doc_model[0].children.iter().map(|ch| ch.value.clone()).collect();
        assert_eq!(expected_text, found_text);
    }

    #[test]
    fn object_blank_lines_are_preserved_or_removed() {
        let input = ["{", "", "    //comment", "    \"w\": true,", "", "    ", "    \"x\": false", "}"].join("\r\n");

        let mut preserve_options = FracturedJsonOptions::default();
        preserve_options.comment_policy = CommentPolicy::Preserve;
        preserve_options.allow_trailing_commas = true;
        preserve_options.preserve_blank_lines = true;

        let preserve_parser = Parser::new(preserve_options);
        let preserve_doc_model = preserve_parser.parse_top_level(&input, false).unwrap();

        assert_eq!(preserve_doc_model.len(), 1);
        assert_eq!(preserve_doc_model[0].item_type, JsonItemType::Object);
        let preserve_expected_types = vec![
            JsonItemType::BlankLine,
            JsonItemType::LineComment,
            JsonItemType::True,
            JsonItemType::BlankLine,
            JsonItemType::BlankLine,
            JsonItemType::False,
        ];
        let preserve_found_types: Vec<JsonItemType> =
            preserve_doc_model[0].children.iter().map(|ch| ch.item_type).collect();
        assert_eq!(preserve_expected_types, preserve_found_types);

        let mut remove_options = FracturedJsonOptions::default();
        remove_options.comment_policy = CommentPolicy::Remove;
        remove_options.allow_trailing_commas = true;
        remove_options.preserve_blank_lines = false;

        let remove_parser = Parser::new(remove_options);
        let remove_doc_model = remove_parser.parse_top_level(&input, false).unwrap();

        assert_eq!(remove_doc_model.len(), 1);
        assert_eq!(remove_doc_model[0].item_type, JsonItemType::Object);
        let remove_expected_types = vec![JsonItemType::True, JsonItemType::False];
        let remove_found_types: Vec<JsonItemType> =
            remove_doc_model[0].children.iter().map(|ch| ch.item_type).collect();
        assert_eq!(remove_expected_types, remove_found_types);
    }

    #[test]
    fn object_with_inline_block_comments() {
        let input = "{ /*a*/ \"w\": /*b*/ 1 /*c*/ }";

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 1);
        assert_eq!(doc_model[0].children[0].prefix_comment, "/*a*/");
        assert_eq!(doc_model[0].children[0].middle_comment, "/*b*/");
        assert_eq!(doc_model[0].children[0].postfix_comment, "/*c*/");
    }

    #[test]
    fn object_middle_comments_combined_1() {
        let input = ["{", "    \"w\" /*a*/ : //b", "        10.9,", "}"].join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 1);
        assert_eq!(doc_model[0].children[0].prefix_comment_length, 0);
        assert_eq!(doc_model[0].children[0].middle_comment, "/*a*/\n//b\n");
        assert_eq!(doc_model[0].children[0].postfix_comment_length, 0);
    }

    #[test]
    fn object_middle_comments_combined_2() {
        let input = ["{", "    \"w\" /*a*/ :", "    /*b*/ 10.9,", "}"].join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 1);
        assert_eq!(doc_model[0].children[0].prefix_comment_length, 0);
        assert_eq!(doc_model[0].children[0].middle_comment, "/*a*/\n/*b*/");
        assert_eq!(doc_model[0].children[0].postfix_comment_length, 0);
    }

    #[test]
    fn object_middle_comments_combined_3() {
        let input = ["{", "    \"w\": //a", "    /*b*/ 10.9,", "}"].join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 1);
        assert_eq!(doc_model[0].children[0].prefix_comment_length, 0);
        assert_eq!(doc_model[0].children[0].middle_comment, "//a\n/*b*/");
        assert_eq!(doc_model[0].children[0].postfix_comment_length, 0);
    }

    #[test]
    fn object_comments_prefer_same_line_elements() {
        let input = [
            "{",
            "          \"w\": 1, /*a*/",
            "    /*b*/ \"x\": 2, /*c*/",
            "          \"y\": 3,  /*d*/",
            "          \"z\": 4",
            "}",
        ]
        .join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 4);
        assert_eq!(doc_model[0].children[0].prefix_comment_length, 0);
        assert_eq!(doc_model[0].children[0].postfix_comment, "/*a*/");
        assert_eq!(doc_model[0].children[1].prefix_comment, "/*b*/");
        assert_eq!(doc_model[0].children[1].postfix_comment, "/*c*/");
        assert_eq!(doc_model[0].children[2].prefix_comment_length, 0);
        assert_eq!(doc_model[0].children[2].postfix_comment, "/*d*/");
    }

    #[test]
    fn object_with_inline_block_comments_2() {
        let input = "{  \"w\": 1, /*a*/ \"x\": 2 }";

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 2);
        assert_eq!(doc_model[0].children[1].prefix_comment, "/*a*/");
    }

    #[test]
    fn object_with_inline_block_comments_3() {
        let input = "{  \"w\": 1, /*a*/ /*b*/ \"x\": 2 }";

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 2);

        assert_eq!(doc_model[0].children[0].name, "\"w\"");
        assert_eq!(doc_model[0].children[0].item_type, JsonItemType::Number);
        assert_eq!(doc_model[0].children[0].postfix_comment, "/*a*/");

        assert_eq!(doc_model[0].children[1].name, "\"x\"");
        assert_eq!(doc_model[0].children[1].item_type, JsonItemType::Number);
        assert_eq!(doc_model[0].children[1].prefix_comment, "/*b*/");
    }

    #[test]
    fn array_comments_for_multiline_element() {
        let input = ["[", "    /*a*/ [", "        1, 2, 3", "    ] //b", "]"].join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 1);
        assert_eq!(doc_model[0].children[0].prefix_comment, "/*a*/");
        assert_eq!(doc_model[0].children[0].postfix_comment, "//b");
    }

    #[test]
    fn object_comments_for_multiline_element() {
        let input = ["{", "    /*a*/ \"w\": [", "        1, 2, 3", "    ] //b", "}"].join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].children.len(), 1);
        assert_eq!(doc_model[0].children[0].prefix_comment, "/*a*/");
        assert_eq!(doc_model[0].children[0].postfix_comment, "//b");
    }

    #[test]
    fn complexity_work() {
        let input = [
            "[",
            "    null,",
            "    [ 1, 2, 3 ],",
            "    [ 1, 2, {}],",
            "    [ 1, 2, { /*a*/ }],",
            "    [ 1, 2, { \"w\": 1 }]",
            "]",
        ]
        .join("\r\n");

        let mut options = FracturedJsonOptions::default();
        options.comment_policy = CommentPolicy::Preserve;
        options.allow_trailing_commas = true;
        options.preserve_blank_lines = true;

        let parser = Parser::new(options);
        let doc_model = parser.parse_top_level(&input, false).unwrap();

        assert_eq!(doc_model.len(), 1);
        assert_eq!(doc_model[0].complexity, 3);
        assert_eq!(doc_model[0].children.len(), 5);

        assert_eq!(doc_model[0].children[0].complexity, 0);
        assert_eq!(doc_model[0].children[1].complexity, 1);
        assert_eq!(doc_model[0].children[2].complexity, 1);
        assert_eq!(doc_model[0].children[2].children[2].complexity, 0);
        assert_eq!(doc_model[0].children[3].complexity, 1);
        assert_eq!(doc_model[0].children[3].children[2].complexity, 0);
        assert_eq!(doc_model[0].children[4].complexity, 2);
        assert_eq!(doc_model[0].children[4].children[2].complexity, 1);
    }

    #[test]
    fn throws_for_malformed_data() {
        let cases = vec![
            "[,1]",
            "[1 2]",
            "[1, 2,]",
            "[1, 2}",
            "[1, 2",
            "[1, /*a*/ 2]",
            "[1, //a\n 2]",
            "{, \"w\":1 }",
            "{ \"w\":1 ",
            "{ /*a*/ \"w\":1 }",
            "{ \"w\":1, }",
            "{ \"w\":1 ]",
            "{ \"w\"::1 ",
            "{ \"w\" \"foo\" }",
            "{ \"w\" {:1 }",
            "{ \"w\":1 \"x\":2 }",
            "{ \"a\": 1, \"b\" }\n",
            "{ \"a\": 1, \"b:\" }\n",
        ];

        let parser = Parser::new(FracturedJsonOptions::default());
        for input in cases {
            assert!(parser.parse_top_level(input, false).is_err(), "input={}", input);
        }
    }

    #[test]
    fn stops_after_first_element() {
        let input = "[ 1, 2 ],[ 3, 4 ]";
        let parser = Parser::new(FracturedJsonOptions::default());
        assert!(parser.parse_top_level(input, true).is_err());
    }
}
