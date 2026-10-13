//! Tests for syntax_error: the parser's message and byte range.

use complexipy_core::cognitive_complexity::syntax_error;

#[test]
fn valid_code_has_no_syntax_error() {
    assert!(syntax_error("def f(x):\n    return x\n").is_none());
}

#[test]
fn reports_the_range_of_the_failing_token() {
    let code = "x = 1\ndef f(:\n    pass\n";
    let error = syntax_error(code).expect("code should not parse");

    assert_eq!(code[..error.start].matches('\n').count(), 1);
    assert!(error.start <= error.end && error.end <= code.len());
    assert!(!error.message.is_empty());
    assert!(!error.message.contains("byte range"));
}

#[test]
fn an_unclosed_bracket_stays_inside_the_code() {
    let code = "values = [1, 2\n";
    let error = syntax_error(code).expect("code should not parse");

    assert!(error.end <= code.len());
}
