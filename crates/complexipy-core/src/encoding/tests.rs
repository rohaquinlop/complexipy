//! Unit tests for `crate::encoding`: PEP 263 coding cookies.

use super::decode_python_source;

#[test]
fn a_file_without_a_cookie_is_utf8() {
    assert_eq!(
        decode_python_source("def f():\n    return \"é\"\n".as_bytes()).unwrap(),
        "def f():\n    return \"é\"\n"
    );
    assert!(decode_python_source(b"x = '\xe9'\n").is_err());
}

#[test]
fn a_utf7_cookie_is_refused_instead_of_read_as_utf8() {
    let source = b"# -*- coding: utf-7 -*-\nok = 1  # +AAo-def evil(): return 1\n";

    let error = decode_python_source(source).unwrap_err();

    assert!(error.contains("utf_7"));
}

#[test]
fn other_stateful_codecs_are_refused() {
    for cookie in [
        "unicode_escape",
        "hz",
        "iso2022_jp",
        "cp037",
        "utf-16",
        "rot13",
    ] {
        let source = format!("# coding: {cookie}\nx = 1\n");

        assert!(decode_python_source(source.as_bytes()).is_err(), "{cookie}");
    }
}

#[test]
fn a_latin1_cookie_decodes_latin1() {
    let source = b"# -*- coding: latin-1 -*-\nname = '\xe9t\xe9'\n";

    assert_eq!(
        decode_python_source(source).unwrap(),
        "# -*- coding: latin-1 -*-\nname = 'été'\n"
    );
}

#[test]
fn an_ascii_compatible_cookie_reads_a_pure_ascii_file() {
    for cookie in [
        "cp1252",
        "iso-8859-15",
        "shift_jis",
        "ascii",
        "utf8",
        "utf-8-unix",
    ] {
        let source = format!("# vim: set fileencoding={cookie} :\nx = 1\n");

        assert_eq!(
            decode_python_source(source.as_bytes()).unwrap(),
            source,
            "{cookie}"
        );
    }
}

#[test]
fn an_ascii_compatible_cookie_refuses_non_ascii_bytes() {
    assert!(decode_python_source(b"# coding: cp1252\nx = '\x93'\n").is_err());
}

#[test]
fn the_cookie_counts_on_the_second_line_only_after_a_comment_or_blank_line() {
    assert!(decode_python_source(b"#!/usr/bin/env python\n# coding: utf-7\nx = 1\n").is_err());
    assert!(decode_python_source(b"\n# coding: utf-7\nx = 1\n").is_err());
    assert!(decode_python_source(b"x = 1\n# coding: utf-7\n").is_ok());
    assert!(decode_python_source(b"x = 1\ny = 2\n# coding: utf-7\n").is_ok());
}

#[test]
fn a_bom_with_another_encoding_is_refused() {
    assert!(decode_python_source(b"\xEF\xBB\xBF# coding: latin-1\nx = 1\n").is_err());
    assert!(decode_python_source(b"\xEF\xBB\xBF# coding: utf-8\nx = 1\n").is_ok());
}
