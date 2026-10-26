use std::sync::OnceLock;

use regex::bytes::Regex;

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

#[derive(Debug, PartialEq, Eq)]
enum Codec {
    Utf8,
    Latin1,
    AsciiCompatible,
}

/// Decodes a Python source file the way CPython does (PEP 263): UTF-8 unless
/// a coding cookie on the first or second line declares otherwise. Latin-1
/// is decoded; a codec whose ASCII bytes decode as ASCII is accepted for a
/// pure-ASCII file; any other declared codec, such as `utf-7`, is an error,
/// because reading that file as UTF-8 would show code CPython never runs and
/// hide code it does.
pub fn decode_python_source(bytes: &[u8]) -> Result<String, String> {
    let (has_bom, body) = match bytes.strip_prefix(UTF8_BOM) {
        Some(body) => (true, body),
        None => (false, bytes),
    };
    let codec = declared_codec(body)?;
    if has_bom && codec != Codec::Utf8 {
        return Err("has a UTF-8 BOM but declares another encoding".to_string());
    }

    match codec {
        Codec::Utf8 => String::from_utf8(bytes.to_vec())
            .map_err(|_| "is not valid UTF-8 and declares no other encoding".to_string()),
        Codec::Latin1 => Ok(body.iter().map(|&byte| char::from(byte)).collect()),
        Codec::AsciiCompatible if body.is_ascii() => {
            Ok(String::from_utf8(body.to_vec()).unwrap_or_default())
        }
        Codec::AsciiCompatible => {
            Err("declares an encoding complexipy decodes only for ASCII text".to_string())
        }
    }
}

fn declared_codec(body: &[u8]) -> Result<Codec, String> {
    let mut lines = body.split(|&byte| byte == b'\n');
    let first = lines.next().unwrap_or_default();
    if let Some(name) = cookie(first) {
        return codec_named(name);
    }
    if !blank_re().is_match(first) {
        return Ok(Codec::Utf8);
    }
    match lines.next().and_then(cookie) {
        Some(name) => codec_named(name),
        None => Ok(Codec::Utf8),
    }
}

fn cookie(line: &[u8]) -> Option<&[u8]> {
    cookie_re()
        .captures(line)
        .and_then(|captures| captures.get(1))
        .map(|name| name.as_bytes())
}

fn cookie_re() -> &'static Regex {
    static COOKIE_RE: OnceLock<Regex> = OnceLock::new();
    COOKIE_RE.get_or_init(|| {
        Regex::new(r"(?-u)^[ \t\x0c]*#.*?coding[:=][ \t]*([-0-9A-Za-z_.]+)").unwrap()
    })
}

fn blank_re() -> &'static Regex {
    static BLANK_RE: OnceLock<Regex> = OnceLock::new();
    BLANK_RE.get_or_init(|| Regex::new(r"(?-u)^[ \t\x0c]*(?:[#\r]|$)").unwrap())
}

fn codec_named(name: &[u8]) -> Result<Codec, String> {
    let name = String::from_utf8_lossy(name)
        .to_ascii_lowercase()
        .replace(['-', ' '], "_");
    if is_utf8(&name) {
        Ok(Codec::Utf8)
    } else if is_latin1(&name) {
        Ok(Codec::Latin1)
    } else if is_ascii_compatible(&name) {
        Ok(Codec::AsciiCompatible)
    } else {
        Err(format!(
            "declares the {:?} encoding, which complexipy cannot decode",
            name
        ))
    }
}

fn is_utf8(name: &str) -> bool {
    matches!(name, "utf_8" | "utf8" | "u8" | "utf" | "cp65001") || name.starts_with("utf_8_")
}

fn is_latin1(name: &str) -> bool {
    matches!(
        name,
        "latin_1"
            | "latin1"
            | "latin"
            | "l1"
            | "iso_8859_1"
            | "iso8859_1"
            | "8859"
            | "cp819"
            | "iso_latin_1"
    ) || ["latin_1_", "iso_8859_1_", "iso_latin_1_"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

fn is_ascii_compatible(name: &str) -> bool {
    const NAMES: [&str; 33] = [
        "ascii",
        "us_ascii",
        "646",
        "us",
        "mac_roman",
        "macroman",
        "macintosh",
        "mac_latin2",
        "maclatin2",
        "mac_cyrillic",
        "maccyrillic",
        "mac_greek",
        "macgreek",
        "mac_iceland",
        "maciceland",
        "mac_turkish",
        "macturkish",
        "euc_jp",
        "eucjp",
        "euc_kr",
        "euckr",
        "gbk",
        "gb2312",
        "gb18030",
        "big5",
        "big5hkscs",
        "shift_jis",
        "shiftjis",
        "sjis",
        "cp932",
        "cp936",
        "cp949",
        "cp950",
    ];
    const PREFIXES: [&str; 6] = [
        "cp125",
        "windows_125",
        "iso8859_",
        "iso_8859_",
        "latin",
        "koi8_",
    ];

    NAMES.contains(&name) || PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests;
