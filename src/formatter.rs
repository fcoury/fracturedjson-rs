use std::sync::Arc;

use crate::buffer::{PaddedFormattingTokens, StringJoinBuffer};
use crate::convert::convert_value_to_dom;
use crate::error::FracturedJsonError;
use crate::model::{BracketPaddingType, JsonItem, JsonItemType, TableColumnType};
use crate::options::{FracturedJsonOptions, TableCommaPlacement};
use crate::parser::Parser;
use crate::table_template::TableTemplate;

/// The main JSON formatter.
///
/// `Formatter` takes JSON input (either as text or Rust values) and produces
/// human-readable, well-formatted output according to the configured options.
///
/// # Example
///
/// ```rust
/// use fracturedjson::Formatter;
///
/// let mut formatter = Formatter::new();
///
/// // Format JSON text
/// let output = formatter.reformat(r#"{"a":1,"b":2}"#, 0).unwrap();
/// assert!(output.contains("\"a\": 1"));
///
/// // Minify JSON
/// let compact = formatter.minify(r#"{ "a": 1, "b": 2 }"#).unwrap();
/// assert_eq!(compact, r#"{"a":1,"b":2}"#);
/// ```
///
/// # Reusing the Formatter
///
/// A single `Formatter` instance can be reused for multiple formatting operations.
/// The configuration in `options` persists across calls, but internal buffers are
/// reset for each operation.
pub struct Formatter {
    /// Configuration options that control formatting behavior.
    /// Modify these before calling formatting methods.
    pub options: FracturedJsonOptions,

    /// Function used to calculate string display width.
    ///
    /// By default, this counts Unicode characters. For accurate alignment with
    /// East Asian wide characters, you can provide a custom function that accounts
    /// for character display widths (e.g., using the `unicode-width` crate).
    ///
    /// # Example
    ///
    /// ```rust
    /// use fracturedjson::Formatter;
    /// use std::sync::Arc;
    ///
    /// let mut formatter = Formatter::new();
    ///
    /// // Use a custom width function (example with simple char count)
    /// formatter.string_length_func = Arc::new(|s: &str| s.chars().count());
    /// ```
    pub string_length_func: Arc<dyn Fn(&str) -> usize + Send + Sync>,
    buffer: StringJoinBuffer,
    pads: PaddedFormattingTokens,
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter {
    /// Creates a new `Formatter` with default options.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fracturedjson::Formatter;
    ///
    /// let formatter = Formatter::new();
    /// ```
    pub fn new() -> Self {
        let options = FracturedJsonOptions::default();
        let string_length_func: Arc<dyn Fn(&str) -> usize + Send + Sync> =
            Arc::new(Self::string_length_by_char_count);
        let pads = PaddedFormattingTokens::new(&options, string_length_func.as_ref());
        Self {
            options,
            string_length_func,
            buffer: StringJoinBuffer::default(),
            pads,
        }
    }

    /// Default string length function that counts Unicode characters.
    ///
    /// This is the default implementation used for calculating display widths.
    /// For most Western text, this produces correct alignment. For text containing
    /// East Asian wide characters, consider using a width-aware function.
    pub fn string_length_by_char_count(value: &str) -> usize {
        value.chars().count()
    }

    /// Reformats JSON text according to the current options.
    ///
    /// Parses the input JSON and produces formatted output with proper indentation,
    /// line breaks, and alignment based on the configured options.
    ///
    /// # Arguments
    ///
    /// * `json_text` - The JSON string to format
    /// * `starting_depth` - Initial indentation depth (usually 0)
    ///
    /// # Returns
    ///
    /// The formatted JSON string, or an error if parsing fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fracturedjson::Formatter;
    ///
    /// let mut formatter = Formatter::new();
    /// let output = formatter.reformat(r#"{"name":"Alice","age":30}"#, 0).unwrap();
    ///
    /// // Output will be nicely formatted with proper spacing
    /// assert!(output.contains("\"name\": \"Alice\""));
    /// ```
    pub fn reformat(
        &mut self,
        json_text: &str,
        starting_depth: usize,
    ) -> Result<String, FracturedJsonError> {
        let parser = Parser::new(self.options.clone());
        let mut doc_model = parser.parse_top_level(json_text, true)?;
        self.format_top_level(&mut doc_model, starting_depth);
        self.buffer.flush();
        Ok(self.buffer.as_string())
    }

    /// Minifies JSON text by removing all unnecessary whitespace.
    ///
    /// Produces the most compact valid JSON representation of the input.
    /// Comments are handled according to `options.comment_policy`.
    ///
    /// # Arguments
    ///
    /// * `json_text` - The JSON string to minify
    ///
    /// # Returns
    ///
    /// The minified JSON string, or an error if parsing fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fracturedjson::Formatter;
    ///
    /// let mut formatter = Formatter::new();
    /// let input = r#"{
    ///     "name": "Alice",
    ///     "age": 30
    /// }"#;
    ///
    /// let output = formatter.minify(input).unwrap();
    /// assert_eq!(output, r#"{"name":"Alice","age":30}"#);
    /// ```
    pub fn minify(&mut self, json_text: &str) -> Result<String, FracturedJsonError> {
        let parser = Parser::new(self.options.clone());
        let mut doc_model = parser.parse_top_level(json_text, true)?;
        self.minify_top_level(&mut doc_model);
        self.buffer.flush();
        Ok(self.buffer.as_string())
    }

    /// Reformats JSONL (JSON Lines) input where each line is a separate JSON value.
    ///
    /// Each line is independently parsed and formatted. Empty lines are preserved.
    /// The output maintains the line structure: one formatted JSON per line.
    ///
    /// # Arguments
    ///
    /// * `jsonl_text` - The JSONL string to format (one JSON value per line)
    ///
    /// # Returns
    ///
    /// The formatted JSONL string, or an error if any line fails to parse.
    /// The error will indicate which line failed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fracturedjson::Formatter;
    ///
    /// let input = r#"{"a":1}
    /// {"b":2}
    /// {"c":3}"#;
    ///
    /// let mut formatter = Formatter::new();
    /// let output = formatter.reformat_jsonl(input).unwrap();
    ///
    /// // Each line is formatted independently
    /// assert!(output.contains("\"a\": 1"));
    /// ```
    pub fn reformat_jsonl(&mut self, jsonl_text: &str) -> Result<String, FracturedJsonError> {
        let mut output_lines = Vec::new();

        for (line_num, line) in jsonl_text.lines().enumerate() {
            // Preserve empty lines
            if line.trim().is_empty() {
                output_lines.push(String::new());
                continue;
            }

            // Format the line
            let formatted = self.reformat(line, 0).map_err(|e| {
                FracturedJsonError::simple(format!("line {}: {}", line_num + 1, e))
            })?;

            // Remove trailing newline since we add our own
            output_lines.push(formatted.trim_end().to_string());
        }

        // Join with newlines and add trailing newline
        let mut result = output_lines.join("\n");
        if !result.is_empty() {
            result.push('\n');
        }
        Ok(result)
    }

    /// Minifies JSONL (JSON Lines) input where each line is a separate JSON value.
    ///
    /// Each line is independently parsed and minified. Empty lines are preserved.
    ///
    /// # Arguments
    ///
    /// * `jsonl_text` - The JSONL string to minify (one JSON value per line)
    ///
    /// # Returns
    ///
    /// The minified JSONL string, or an error if any line fails to parse.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fracturedjson::Formatter;
    ///
    /// let input = r#"{ "a": 1 }
    /// { "b": 2 }"#;
    ///
    /// let mut formatter = Formatter::new();
    /// let output = formatter.minify_jsonl(input).unwrap();
    ///
    /// assert!(output.contains(r#"{"a":1}"#));
    /// ```
    pub fn minify_jsonl(&mut self, jsonl_text: &str) -> Result<String, FracturedJsonError> {
        let mut output_lines = Vec::new();

        for (line_num, line) in jsonl_text.lines().enumerate() {
            // Preserve empty lines
            if line.trim().is_empty() {
                output_lines.push(String::new());
                continue;
            }

            // Minify the line
            let minified = self.minify(line).map_err(|e| {
                FracturedJsonError::simple(format!("line {}: {}", line_num + 1, e))
            })?;

            // Remove trailing newline since we add our own
            output_lines.push(minified.trim_end().to_string());
        }

        // Join with newlines and add trailing newline
        let mut result = output_lines.join("\n");
        if !result.is_empty() {
            result.push('\n');
        }
        Ok(result)
    }

    /// Formats a [`serde_json::Value`] according to the current options.
    ///
    /// This is useful when you already have parsed JSON data and want to
    /// format it without going through text parsing again.
    ///
    /// # Arguments
    ///
    /// * `value` - The JSON value to format
    /// * `starting_depth` - Initial indentation depth (usually 0)
    /// * `recursion_limit` - Maximum nesting depth to prevent stack overflow
    ///
    /// # Returns
    ///
    /// The formatted JSON string, or an error if the recursion limit is exceeded.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fracturedjson::Formatter;
    /// use serde_json::json;
    ///
    /// let mut formatter = Formatter::new();
    /// let value = json!({"name": "Alice", "scores": [95, 87, 92]});
    ///
    /// let output = formatter.serialize_value(&value, 0, 100).unwrap();
    /// ```
    pub fn serialize_value(
        &mut self,
        value: &serde_json::Value,
        starting_depth: usize,
        recursion_limit: usize,
    ) -> Result<String, FracturedJsonError> {
        let doc_model = convert_value_to_dom(value, None, recursion_limit)?;
        let mut doc_list = Vec::new();
        if let Some(item) = doc_model {
            doc_list.push(item);
        }
        self.format_top_level(&mut doc_list, starting_depth);
        self.buffer.flush();
        Ok(self.buffer.as_string())
    }

    /// Serializes any [`serde::Serialize`] type to formatted JSON.
    ///
    /// This is the most convenient method for formatting Rust data structures.
    /// The value is first converted to a `serde_json::Value`, then formatted.
    ///
    /// # Arguments
    ///
    /// * `value` - Any value implementing `Serialize`
    /// * `starting_depth` - Initial indentation depth (usually 0)
    /// * `recursion_limit` - Maximum nesting depth to prevent stack overflow
    ///
    /// # Returns
    ///
    /// The formatted JSON string, or an error if serialization fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fracturedjson::Formatter;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Person {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// let person = Person {
    ///     name: "Alice".into(),
    ///     age: 30,
    /// };
    ///
    /// let mut formatter = Formatter::new();
    /// let output = formatter.serialize(&person, 0, 100).unwrap();
    ///
    /// assert!(output.contains("\"name\": \"Alice\""));
    /// ```
    pub fn serialize<T: serde::Serialize>(
        &mut self,
        value: &T,
        starting_depth: usize,
        recursion_limit: usize,
    ) -> Result<String, FracturedJsonError> {
        let json_value = serde_json::to_value(value).map_err(|err| {
            FracturedJsonError::simple(format!("Failed to serialize value: {}", err))
        })?;
        self.serialize_value(&json_value, starting_depth, recursion_limit)
    }

    fn format_top_level(&mut self, doc_model: &mut [JsonItem], starting_depth: usize) {
        self.buffer = StringJoinBuffer::default();
        self.pads = PaddedFormattingTokens::new(&self.options, self.string_length_func.as_ref());

        for item in doc_model.iter_mut() {
            self.compute_item_lengths(item);
            self.format_item(item, starting_depth, false, None);
        }
    }

    fn minify_top_level(&mut self, doc_model: &mut [JsonItem]) {
        self.buffer = StringJoinBuffer::default();
        self.pads = PaddedFormattingTokens::new(&self.options, self.string_length_func.as_ref());

        let mut at_start_of_new_line = true;
        for item in doc_model.iter() {
            at_start_of_new_line = self.minify_item(item, at_start_of_new_line);
        }
    }

    fn compute_item_lengths(&mut self, item: &mut JsonItem) {
        for child in item.children.iter_mut() {
            self.compute_item_lengths(child);
        }

        item.value_length = match item.item_type {
            JsonItemType::Null => self.pads.literal_null_len(),
            JsonItemType::True => self.pads.literal_true_len(),
            JsonItemType::False => self.pads.literal_false_len(),
            _ => (self.string_length_func)(&item.value),
        };

        item.name_length = (self.string_length_func)(&item.name);
        item.prefix_comment_length = (self.string_length_func)(&item.prefix_comment);
        item.middle_comment_length = (self.string_length_func)(&item.middle_comment);
        item.postfix_comment_length = (self.string_length_func)(&item.postfix_comment);

        let newline = "\n";
        item.requires_multiple_lines = matches!(
            item.item_type,
            JsonItemType::BlankLine | JsonItemType::BlockComment | JsonItemType::LineComment
        ) || item
            .children
            .iter()
            .any(|ch| ch.requires_multiple_lines || ch.is_post_comment_line_style)
            || item.prefix_comment.contains(newline)
            || item.middle_comment.contains(newline)
            || item.postfix_comment.contains(newline)
            || item.value.contains(newline);

        if matches!(item.item_type, JsonItemType::Array | JsonItemType::Object) {
            let pad_type = Self::get_padding_type(item);
            let children_len: usize = item.children.iter().map(|ch| ch.minimum_total_length).sum();
            let commas = self
                .pads
                .comma_len()
                .saturating_mul(item.children.len().saturating_sub(1));
            item.value_length = self.pads.start_len(item.item_type, pad_type)
                + self.pads.end_len(item.item_type, pad_type)
                + children_len
                + commas;
        }

        item.minimum_total_length = if item.prefix_comment_length > 0 {
            item.prefix_comment_length + self.pads.comment_len()
        } else {
            0
        } + if item.name_length > 0 {
            item.name_length + self.pads.colon_len()
        } else {
            0
        } + if item.middle_comment_length > 0 {
            item.middle_comment_length + self.pads.comment_len()
        } else {
            0
        } + item.value_length
            + if item.postfix_comment_length > 0 {
                item.postfix_comment_length + self.pads.comment_len()
            } else {
                0
            };
    }

    fn format_item(
        &mut self,
        item: &JsonItem,
        depth: usize,
        include_trailing_comma: bool,
        parent_template: Option<&TableTemplate>,
    ) {
        match item.item_type {
            JsonItemType::Array | JsonItemType::Object => {
                self.format_container(item, depth, include_trailing_comma, parent_template)
            }
            JsonItemType::BlankLine => self.format_blank_line(),
            JsonItemType::BlockComment | JsonItemType::LineComment => {
                self.format_standalone_comment(item, depth)
            }
            _ => {
                if item.requires_multiple_lines {
                    self.format_split_key_value(
                        item,
                        depth,
                        include_trailing_comma,
                        parent_template,
                    );
                } else {
                    self.format_inline_element(
                        item,
                        depth,
                        include_trailing_comma,
                        parent_template,
                    );
                }
            }
        }
    }

    fn format_container(
        &mut self,
        item: &JsonItem,
        depth: usize,
        include_trailing_comma: bool,
        parent_template: Option<&TableTemplate>,
    ) {
        if (depth as isize) > self.options.always_expand_depth
            && self.format_container_inline(item, depth, include_trailing_comma, parent_template)
        {
            return;
        }

        let item_complexity = item.complexity as isize;
        let recursive_template = item_complexity <= self.options.max_compact_array_complexity
            || item_complexity <= self.options.max_table_row_complexity + 1;
        let mut template =
            TableTemplate::new(self.pads.clone(), self.options.number_list_alignment);
        template.measure_table_root(item, recursive_template);

        if (depth as isize) > self.options.always_expand_depth
            && self.format_container_compact_multiline(
                item,
                depth,
                include_trailing_comma,
                &template,
                parent_template,
            )
        {
            return;
        }

        if (depth as isize) >= self.options.always_expand_depth {
            let mut table_template = template.clone();
            if self.format_container_table(
                item,
                depth,
                include_trailing_comma,
                &mut table_template,
                parent_template,
            ) {
                return;
            }
        }

        self.format_container_expanded(
            item,
            depth,
            include_trailing_comma,
            &template,
            parent_template,
        );
    }

    fn format_container_inline(
        &mut self,
        item: &JsonItem,
        depth: usize,
        include_trailing_comma: bool,
        parent_template: Option<&TableTemplate>,
    ) -> bool {
        if item.requires_multiple_lines {
            return false;
        }

        let (prefix_length, name_length) = if let Some(parent) = parent_template {
            let prefix = if parent.prefix_comment_length > 0 {
                parent.prefix_comment_length + self.pads.comment_len()
            } else {
                0
            };
            let name = if parent.name_length > 0 {
                parent.name_length + self.pads.colon_len()
            } else {
                0
            };
            (prefix, name)
        } else {
            let prefix = if item.prefix_comment_length > 0 {
                item.prefix_comment_length + self.pads.comment_len()
            } else {
                0
            };
            let name = if item.name_length > 0 {
                item.name_length + self.pads.colon_len()
            } else {
                0
            };
            (prefix, name)
        };

        let length_to_consider = prefix_length
            + name_length
            + if item.middle_comment_length > 0 {
                item.middle_comment_length + self.pads.comment_len()
            } else {
                0
            }
            + item.value_length
            + if item.postfix_comment_length > 0 {
                item.postfix_comment_length + self.pads.comment_len()
            } else {
                0
            }
            + if include_trailing_comma {
                self.pads.comma_len()
            } else {
                0
            };

        if (item.complexity as isize) > self.options.max_inline_complexity
            || length_to_consider > self.available_line_space(depth)
        {
            return false;
        }

        let indent = self.pads.indent(depth);
        self.buffer.add(&self.options.prefix_string).add(&indent);
        self.inline_element(item, include_trailing_comma, parent_template);
        self.buffer.end_line(self.pads.eol());
        true
    }

    fn format_container_compact_multiline(
        &mut self,
        item: &JsonItem,
        depth: usize,
        include_trailing_comma: bool,
        template: &TableTemplate,
        parent_template: Option<&TableTemplate>,
    ) -> bool {
        if item.item_type != JsonItemType::Array {
            return false;
        }
        if item.children.is_empty()
            || item.children.len() < self.options.min_compact_array_row_items
        {
            return false;
        }
        if (item.complexity as isize) > self.options.max_compact_array_complexity {
            return false;
        }
        if item.requires_multiple_lines {
            return false;
        }

        let use_table_formatting = !matches!(
            template.column_type,
            TableColumnType::Unknown | TableColumnType::Mixed
        );
        let likely_available_line_space = self.available_line_space(depth + 1);

        let mut avg_item_width = self.pads.comma_len();
        if use_table_formatting {
            avg_item_width += template.total_length;
        } else {
            let sum: usize = item.children.iter().map(|ch| ch.minimum_total_length).sum();
            avg_item_width += sum / item.children.len().max(1);
        }
        if avg_item_width * self.options.min_compact_array_row_items > likely_available_line_space {
            return false;
        }

        let depth_after_colon = self.standard_format_start(item, depth, parent_template);
        self.buffer
            .add(self.pads.start(item.item_type, BracketPaddingType::Empty));

        let available_line_space = self.available_line_space(depth_after_colon + 1);
        let mut remaining_line_space: isize = -1;
        for (i, child) in item.children.iter().enumerate() {
            let needs_comma = i < item.children.len() - 1;
            let space_needed = if use_table_formatting {
                (if needs_comma {
                    self.pads.comma_len()
                } else {
                    0
                }) + template.total_length
            } else {
                (if needs_comma {
                    self.pads.comma_len()
                } else {
                    0
                }) + child.minimum_total_length
            };

            if remaining_line_space < space_needed as isize {
                let indent = self.pads.indent(depth_after_colon + 1);
                self.buffer
                    .end_line(self.pads.eol())
                    .add(&self.options.prefix_string)
                    .add(&indent);
                remaining_line_space = available_line_space as isize;
            }

            if use_table_formatting {
                self.inline_table_row_segment(template, child, needs_comma, false);
            } else {
                self.inline_element(child, needs_comma, None);
            }
            remaining_line_space -= space_needed as isize;
        }

        let indent = self.pads.indent(depth_after_colon);
        self.buffer
            .end_line(self.pads.eol())
            .add(&self.options.prefix_string)
            .add(&indent)
            .add(self.pads.end(item.item_type, BracketPaddingType::Empty));
        self.standard_format_end(item, include_trailing_comma);
        true
    }

    fn format_container_table(
        &mut self,
        item: &JsonItem,
        depth: usize,
        include_trailing_comma: bool,
        template: &mut TableTemplate,
        parent_template: Option<&TableTemplate>,
    ) -> bool {
        if (item.complexity as isize) > self.options.max_table_row_complexity + 1 {
            return false;
        }
        if template.requires_multiple_lines {
            return false;
        }

        let available_space_depth = if item.middle_comment_has_new_line {
            depth + 2
        } else {
            depth + 1
        };
        let available_space = self
            .available_line_space(available_space_depth)
            .saturating_sub(self.pads.comma_len());

        let is_child_too_long = item
            .children
            .iter()
            .filter(|ch| !Self::is_comment_or_blank_line(ch.item_type))
            .any(|ch| ch.minimum_total_length > available_space);
        if is_child_too_long {
            return false;
        }

        if template.column_type == TableColumnType::Mixed || !template.try_to_fit(available_space) {
            return false;
        }

        let depth_after_colon = self.standard_format_start(item, depth, parent_template);
        self.buffer
            .add(self.pads.start(item.item_type, BracketPaddingType::Empty))
            .end_line(self.pads.eol());

        let last_element_index = Self::index_of_last_element(&item.children);
        for (i, row_item) in item.children.iter().enumerate() {
            match row_item.item_type {
                JsonItemType::BlankLine => {
                    self.format_blank_line();
                    continue;
                }
                JsonItemType::LineComment | JsonItemType::BlockComment => {
                    self.format_standalone_comment(row_item, depth_after_colon + 1);
                    continue;
                }
                _ => {}
            }

            let indent = self.pads.indent(depth_after_colon + 1);
            self.buffer.add(&self.options.prefix_string).add(&indent);
            self.inline_table_row_segment(
                template,
                row_item,
                (i as isize) < last_element_index,
                true,
            );
            self.buffer.end_line(self.pads.eol());
        }

        let indent = self.pads.indent(depth_after_colon);
        self.buffer
            .add(&self.options.prefix_string)
            .add(&indent)
            .add(self.pads.end(item.item_type, BracketPaddingType::Empty));
        self.standard_format_end(item, include_trailing_comma);
        true
    }

    fn format_container_expanded(
        &mut self,
        item: &JsonItem,
        depth: usize,
        include_trailing_comma: bool,
        template: &TableTemplate,
        parent_template: Option<&TableTemplate>,
    ) {
        let depth_after_colon = self.standard_format_start(item, depth, parent_template);
        self.buffer
            .add(self.pads.start(item.item_type, BracketPaddingType::Empty))
            .end_line(self.pads.eol());

        let align_props = item.item_type == JsonItemType::Object
            && template.name_length.saturating_sub(template.name_minimum)
                <= self.options.max_prop_name_padding
            && !template.any_middle_comment_has_newline
            && self.available_line_space(depth + 1) >= template.atomic_item_size();
        let template_to_pass = if align_props { Some(template) } else { None };

        let last_element_index = Self::index_of_last_element(&item.children);
        for (i, child) in item.children.iter().enumerate() {
            self.format_item(
                child,
                depth_after_colon + 1,
                (i as isize) < last_element_index,
                template_to_pass,
            );
        }

        let indent = self.pads.indent(depth_after_colon);
        self.buffer
            .add(&self.options.prefix_string)
            .add(&indent)
            .add(self.pads.end(item.item_type, BracketPaddingType::Empty));
        self.standard_format_end(item, include_trailing_comma);
    }

    fn format_standalone_comment(&mut self, item: &JsonItem, depth: usize) {
        let comment_rows =
            Self::normalize_multiline_comment(&item.value, item.input_position.column);
        let indent = self.pads.indent(depth);
        for line in comment_rows {
            self.buffer
                .add(&self.options.prefix_string)
                .add(&indent)
                .add(&line)
                .end_line(self.pads.eol());
        }
    }

    fn format_blank_line(&mut self) {
        self.buffer
            .add(&self.options.prefix_string)
            .end_line(self.pads.eol());
    }

    fn format_inline_element(
        &mut self,
        item: &JsonItem,
        depth: usize,
        include_trailing_comma: bool,
        parent_template: Option<&TableTemplate>,
    ) {
        let indent = self.pads.indent(depth);
        self.buffer.add(&self.options.prefix_string).add(&indent);
        self.inline_element(item, include_trailing_comma, parent_template);
        self.buffer.end_line(self.pads.eol());
    }

    fn format_split_key_value(
        &mut self,
        item: &JsonItem,
        depth: usize,
        include_trailing_comma: bool,
        parent_template: Option<&TableTemplate>,
    ) {
        self.standard_format_start(item, depth, parent_template);
        self.buffer.add(&item.value);
        self.standard_format_end(item, include_trailing_comma);
    }

    fn standard_format_start(
        &mut self,
        item: &JsonItem,
        depth: usize,
        parent_template: Option<&TableTemplate>,
    ) -> usize {
        let indent = self.pads.indent(depth);
        self.buffer.add(&self.options.prefix_string).add(&indent);

        let comment_sep = self.pads.comment().to_string();
        let colon_sep = self.pads.colon().to_string();

        if let Some(parent) = parent_template {
            self.add_to_buffer_fixed(
                &item.prefix_comment,
                item.prefix_comment_length,
                parent.prefix_comment_length,
                &comment_sep,
                false,
            );
            self.add_to_buffer_fixed(
                &item.name,
                item.name_length,
                parent.name_length,
                &colon_sep,
                self.options.colon_before_prop_name_padding,
            );
        } else {
            self.add_to_buffer(
                &item.prefix_comment,
                item.prefix_comment_length,
                &comment_sep,
            );
            self.add_to_buffer(&item.name, item.name_length, &colon_sep);
        }

        if item.middle_comment_length == 0 {
            return depth;
        }

        if !item.middle_comment_has_new_line {
            let middle_pad = parent_template
                .map(|parent| {
                    parent
                        .middle_comment_length
                        .saturating_sub(item.middle_comment_length)
                })
                .unwrap_or(0);
            self.buffer
                .add(&item.middle_comment)
                .spaces(middle_pad)
                .add(self.pads.comment());
            return depth;
        }

        let comment_rows = Self::normalize_multiline_comment(&item.middle_comment, usize::MAX);
        self.buffer.end_line(self.pads.eol());
        let indent = self.pads.indent(depth + 1);
        for row in comment_rows {
            self.buffer
                .add(&self.options.prefix_string)
                .add(&indent)
                .add(&row)
                .end_line(self.pads.eol());
        }
        let indent = self.pads.indent(depth + 1);
        self.buffer.add(&self.options.prefix_string).add(&indent);
        depth + 1
    }

    fn standard_format_end(&mut self, item: &JsonItem, include_trailing_comma: bool) {
        if include_trailing_comma && item.is_post_comment_line_style {
            self.buffer.add(self.pads.comma());
        }
        if item.postfix_comment_length > 0 {
            self.buffer
                .add(self.pads.comment())
                .add(&item.postfix_comment);
        }
        if include_trailing_comma && !item.is_post_comment_line_style {
            self.buffer.add(self.pads.comma());
        }
        self.buffer.end_line(self.pads.eol());
    }

    fn inline_element(
        &mut self,
        item: &JsonItem,
        include_trailing_comma: bool,
        parent_template: Option<&TableTemplate>,
    ) {
        if item.requires_multiple_lines {
            return;
        }

        let comment_sep = self.pads.comment().to_string();
        let colon_sep = self.pads.colon().to_string();

        if let Some(parent) = parent_template {
            self.add_to_buffer_fixed(
                &item.prefix_comment,
                item.prefix_comment_length,
                parent.prefix_comment_length,
                &comment_sep,
                false,
            );
            self.add_to_buffer_fixed(
                &item.name,
                item.name_length,
                parent.name_length,
                &colon_sep,
                self.options.colon_before_prop_name_padding,
            );
            self.add_to_buffer_fixed(
                &item.middle_comment,
                item.middle_comment_length,
                parent.middle_comment_length,
                &comment_sep,
                false,
            );
        } else {
            self.add_to_buffer(
                &item.prefix_comment,
                item.prefix_comment_length,
                &comment_sep,
            );
            self.add_to_buffer(&item.name, item.name_length, &colon_sep);
            self.add_to_buffer(
                &item.middle_comment,
                item.middle_comment_length,
                &comment_sep,
            );
        }

        self.inline_element_raw(item);

        if include_trailing_comma && item.is_post_comment_line_style {
            self.buffer.add(self.pads.comma());
        }
        if item.postfix_comment_length > 0 {
            self.buffer
                .add(self.pads.comment())
                .add(&item.postfix_comment);
        }
        if include_trailing_comma && !item.is_post_comment_line_style {
            self.buffer.add(self.pads.comma());
        }
    }

    fn inline_element_raw(&mut self, item: &JsonItem) {
        match item.item_type {
            JsonItemType::Array => {
                let pad_type = Self::get_padding_type(item);
                self.buffer.add(self.pads.arr_start(pad_type));
                for (i, child) in item.children.iter().enumerate() {
                    self.inline_element(child, i < item.children.len() - 1, None);
                }
                self.buffer.add(self.pads.arr_end(pad_type));
            }
            JsonItemType::Object => {
                let pad_type = Self::get_padding_type(item);
                self.buffer.add(self.pads.obj_start(pad_type));
                for (i, child) in item.children.iter().enumerate() {
                    self.inline_element(child, i < item.children.len() - 1, None);
                }
                self.buffer.add(self.pads.obj_end(pad_type));
            }
            _ => {
                self.buffer.add(&item.value);
            }
        }
    }

    fn inline_table_row_segment(
        &mut self,
        template: &TableTemplate,
        item: &JsonItem,
        include_trailing_comma: bool,
        is_whole_row: bool,
    ) {
        let comment_sep = self.pads.comment().to_string();
        let colon_sep = self.pads.colon().to_string();

        self.add_to_buffer_fixed(
            &item.prefix_comment,
            item.prefix_comment_length,
            template.prefix_comment_length,
            &comment_sep,
            false,
        );
        self.add_to_buffer_fixed(
            &item.name,
            item.name_length,
            template.name_length,
            &colon_sep,
            self.options.colon_before_prop_name_padding,
        );
        self.add_to_buffer_fixed(
            &item.middle_comment,
            item.middle_comment_length,
            template.middle_comment_length,
            &comment_sep,
            false,
        );

        let comma_before_pad = self.options.table_comma_placement
            == TableCommaPlacement::BeforePadding
            || (self.options.table_comma_placement
                == TableCommaPlacement::BeforePaddingExceptNumbers
                && template.column_type != TableColumnType::Number);

        let comma_pos =
            if template.postfix_comment_length > 0 && !template.is_any_post_comment_line_style {
                if item.postfix_comment_length > 0 {
                    if comma_before_pad {
                        CommaPosition::BeforeCommentPadding
                    } else {
                        CommaPosition::AfterCommentPadding
                    }
                } else if comma_before_pad {
                    CommaPosition::BeforeValuePadding
                } else {
                    CommaPosition::AfterCommentPadding
                }
            } else if comma_before_pad {
                CommaPosition::BeforeValuePadding
            } else {
                CommaPosition::AfterValuePadding
            };

        let comma_type = if include_trailing_comma {
            self.pads.comma().to_string()
        } else if is_whole_row {
            self.pads.dummy_comma().to_string()
        } else {
            String::new()
        };

        if !template.children.is_empty() && item.item_type != JsonItemType::Null {
            if template.column_type == TableColumnType::Array {
                self.inline_table_raw_array(template, item);
            } else {
                self.inline_table_raw_object(template, item);
            }
            if matches!(comma_pos, CommaPosition::BeforeValuePadding) {
                self.buffer.add(&comma_type);
            }
            if template.shorter_than_null_adjustment > 0 {
                self.buffer.spaces(template.shorter_than_null_adjustment);
            }
        } else if template.column_type == TableColumnType::Number {
            let number_comma_type = if matches!(comma_pos, CommaPosition::BeforeValuePadding) {
                comma_type.as_str()
            } else {
                ""
            };
            template.format_number(&mut self.buffer, item, number_comma_type);
        } else {
            self.inline_element_raw(item);
            if matches!(comma_pos, CommaPosition::BeforeValuePadding) {
                self.buffer.add(&comma_type);
            }
            self.buffer
                .spaces(template.composite_value_length - item.value_length);
        }

        if matches!(comma_pos, CommaPosition::AfterValuePadding) {
            self.buffer.add(&comma_type);
        }

        if template.postfix_comment_length > 0 {
            self.buffer
                .add(self.pads.comment())
                .add(&item.postfix_comment);
        }

        if matches!(comma_pos, CommaPosition::BeforeCommentPadding) {
            self.buffer.add(&comma_type);
        }

        self.buffer.spaces(
            template
                .postfix_comment_length
                .saturating_sub(item.postfix_comment_length),
        );

        if matches!(comma_pos, CommaPosition::AfterCommentPadding) {
            self.buffer.add(&comma_type);
        }
    }

    fn inline_table_raw_array(&mut self, template: &TableTemplate, item: &JsonItem) {
        self.buffer.add(self.pads.arr_start(template.pad_type));
        for (i, sub_template) in template.children.iter().enumerate() {
            let is_last_in_template = i == template.children.len() - 1;
            let is_last_in_array = i == item.children.len().saturating_sub(1);
            let is_past_end = i >= item.children.len();

            if is_past_end {
                self.buffer.spaces(sub_template.total_length);
                if !is_last_in_template {
                    self.buffer.add(self.pads.dummy_comma());
                }
            } else {
                self.inline_table_row_segment(
                    sub_template,
                    &item.children[i],
                    !is_last_in_array,
                    false,
                );
                if is_last_in_array && !is_last_in_template {
                    self.buffer.add(self.pads.dummy_comma());
                }
            }
        }
        self.buffer.add(self.pads.arr_end(template.pad_type));
    }

    fn inline_table_raw_object(&mut self, template: &TableTemplate, item: &JsonItem) {
        let mut matches: Vec<(&TableTemplate, Option<&JsonItem>)> = Vec::new();
        for sub in &template.children {
            let matched = item
                .children
                .iter()
                .find(|ch| ch.name == sub.location_in_parent.clone().unwrap_or_default());
            matches.push((sub, matched));
        }

        let mut last_non_null_idx: isize = matches.len() as isize - 1;
        while last_non_null_idx >= 0 && matches[last_non_null_idx as usize].1.is_none() {
            last_non_null_idx -= 1;
        }

        self.buffer.add(self.pads.obj_start(template.pad_type));
        for (i, (sub_template, sub_item)) in matches.iter().enumerate() {
            let is_last_in_object = i as isize == last_non_null_idx;
            let is_last_in_template = i == matches.len() - 1;

            if let Some(item) = sub_item {
                self.inline_table_row_segment(sub_template, item, !is_last_in_object, false);
                if is_last_in_object && !is_last_in_template {
                    self.buffer.add(self.pads.dummy_comma());
                }
            } else {
                self.buffer.spaces(sub_template.total_length);
                if !is_last_in_template {
                    self.buffer.add(self.pads.dummy_comma());
                }
            }
        }
        self.buffer.add(self.pads.obj_end(template.pad_type));
    }

    fn available_line_space(&self, depth: usize) -> usize {
        self.options
            .max_total_line_length
            .saturating_sub(self.pads.prefix_string_len())
            .saturating_sub(self.options.indent_spaces.saturating_mul(depth))
    }

    fn minify_item(&mut self, item: &JsonItem, at_start_of_new_line: bool) -> bool {
        let newline = "\n";
        self.buffer.add(&item.prefix_comment);
        if !item.name.is_empty() {
            self.buffer.add(&item.name).add(":");
        }

        if item.middle_comment.contains(newline) {
            let normalized = Self::normalize_multiline_comment(&item.middle_comment, usize::MAX);
            for line in normalized {
                self.buffer.add(&line).add(newline);
            }
        } else {
            self.buffer.add(&item.middle_comment);
        }

        match item.item_type {
            JsonItemType::Array | JsonItemType::Object => {
                let close_bracket = if item.item_type == JsonItemType::Array {
                    self.buffer.add("[");
                    "]"
                } else {
                    self.buffer.add("{");
                    "}"
                };

                let mut needs_comma = false;
                let mut at_start = false;
                for child in &item.children {
                    if !Self::is_comment_or_blank_line(child.item_type) {
                        if needs_comma {
                            self.buffer.add(",");
                        }
                        needs_comma = true;
                    }
                    at_start = self.minify_item(child, at_start);
                }
                self.buffer.add(close_bracket);
            }
            JsonItemType::BlankLine => {
                if !at_start_of_new_line {
                    self.buffer.add(newline);
                }
                self.buffer.add(newline);
                return true;
            }
            JsonItemType::LineComment => {
                if !at_start_of_new_line {
                    self.buffer.add(newline);
                }
                self.buffer.add(&item.value).add(newline);
                return true;
            }
            JsonItemType::BlockComment => {
                if !at_start_of_new_line {
                    self.buffer.add(newline);
                }

                if item.value.contains(newline) {
                    let normalized =
                        Self::normalize_multiline_comment(&item.value, item.input_position.column);
                    for line in normalized {
                        self.buffer.add(&line).add(newline);
                    }
                    return true;
                }

                self.buffer.add(&item.value).add(newline);
                return true;
            }
            _ => {
                self.buffer.add(&item.value);
            }
        }

        self.buffer.add(&item.postfix_comment);
        if !item.postfix_comment.is_empty() && item.is_post_comment_line_style {
            self.buffer.add(newline);
            return true;
        }

        false
    }

    fn add_to_buffer(&mut self, value: &str, value_width: usize, separator: &str) {
        if value_width == 0 {
            return;
        }
        self.buffer.add(value).add(separator);
    }

    fn add_to_buffer_fixed(
        &mut self,
        value: &str,
        value_width: usize,
        field_width: usize,
        separator: &str,
        separator_before_padding: bool,
    ) {
        if field_width == 0 {
            return;
        }
        let pad_width = field_width.saturating_sub(value_width);
        if separator_before_padding {
            self.buffer.add(value).add(separator).spaces(pad_width);
        } else {
            self.buffer.add(value).spaces(pad_width).add(separator);
        }
    }

    fn get_padding_type(arr_or_obj: &JsonItem) -> BracketPaddingType {
        if arr_or_obj.children.is_empty() {
            return BracketPaddingType::Empty;
        }
        if arr_or_obj.complexity >= 2 {
            BracketPaddingType::Complex
        } else {
            BracketPaddingType::Simple
        }
    }

    fn normalize_multiline_comment(comment: &str, first_line_column: usize) -> Vec<String> {
        let normalized = comment.replace('\r', "");
        let mut comment_rows: Vec<String> = normalized
            .split('\n')
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();

        for line in comment_rows.iter_mut().skip(1) {
            let mut non_ws_idx = 0usize;
            for (seen, (idx, ch)) in line.char_indices().enumerate() {
                if seen >= first_line_column {
                    break;
                }
                if !ch.is_whitespace() {
                    break;
                }
                non_ws_idx = idx + ch.len_utf8();
            }
            *line = line[non_ws_idx..].to_string();
        }

        comment_rows
    }

    fn index_of_last_element(item_list: &[JsonItem]) -> isize {
        for (i, item) in item_list.iter().enumerate().rev() {
            if !Self::is_comment_or_blank_line(item.item_type) {
                return i as isize;
            }
        }
        -1
    }

    fn is_comment_or_blank_line(item_type: JsonItemType) -> bool {
        matches!(
            item_type,
            JsonItemType::BlankLine | JsonItemType::BlockComment | JsonItemType::LineComment
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
enum CommaPosition {
    BeforeValuePadding,
    AfterValuePadding,
    BeforeCommentPadding,
    AfterCommentPadding,
}
