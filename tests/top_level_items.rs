use fracturedjson::{CommentPolicy, Formatter};

#[test]
fn error_if_multiple_top_level_elements() {
    let input = "[1,2] [3,4]";
    let mut formatter = Formatter::new();
    assert!(formatter.reformat(input, 0).is_err());
    assert!(formatter.minify(input).is_err());
}

#[test]
fn error_if_multiple_top_level_elements_with_comma() {
    let input = "[1,2], [3,4]";
    let mut formatter = Formatter::new();
    assert!(formatter.reformat(input, 0).is_err());
    assert!(formatter.minify(input).is_err());
}

#[test]
fn comments_after_top_level_element_are_preserved() {
    let input = "/*a*/ [1,2] /*b*/ //c";
    let mut formatter = Formatter::new();
    formatter.options.comment_policy = CommentPolicy::Preserve;
    let output = formatter.reformat(input, 0).unwrap();

    assert!(output.contains("/*a*/"));
    assert!(output.contains("/*b*/"));
    assert!(output.contains("//c"));

    let minified_output = formatter.reformat(input, 0).unwrap();
    assert!(minified_output.contains("/*a*/"));
    assert!(minified_output.contains("/*b*/"));
    assert!(minified_output.contains("//c"));
}
