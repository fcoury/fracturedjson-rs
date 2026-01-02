pub fn do_instances_line_up(lines: &[String], substring: &str) -> bool {
    let mut indices: Vec<usize> = lines
        .iter()
        .filter_map(|line| find_char_index(line, substring))
        .collect();
    if indices.is_empty() {
        return true;
    }
    indices.dedup();
    indices.len() == 1
}

pub fn find_char_index(haystack: &str, needle: &str) -> Option<usize> {
    haystack.find(needle).map(|byte_idx| haystack[..byte_idx].chars().count())
}

pub fn normalize_quotes(input: &str) -> String {
    input.replace('\'', "\"")
}

pub fn join_lines(lines: &[&str]) -> String {
    lines.join("\n")
}

pub fn split_trimmed_lines(text: &str, eol: &str) -> Vec<String> {
    text.trim_end()
        .split(eol)
        .map(|s| s.to_string())
        .collect()
}
