use std::collections::HashMap;

use lsp_types::Uri;

#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Uri,
    pub text: String,
    pub version: i32,
}

#[derive(Debug, Default)]
pub struct Documents {
    open: HashMap<String, Document>,
}

impl Documents {
    pub fn key(uri: &Uri) -> String {
        uri.as_str().to_string()
    }

    pub fn open(&mut self, uri: Uri, text: String, version: i32) {
        self.open
            .insert(Self::key(&uri), Document { uri, text, version });
    }

    pub fn change(&mut self, uri: &Uri, text: String, version: i32) {
        if let Some(document) = self.open.get_mut(&Self::key(uri)) {
            document.text = text;
            document.version = version;
        }
    }

    pub fn close(&mut self, uri: &Uri) {
        self.open.remove(&Self::key(uri));
    }

    pub fn get(&self, uri: &Uri) -> Option<&Document> {
        self.open.get(&Self::key(uri))
    }

    pub fn get_keyed(&self, key: &str) -> Option<&Document> {
        self.open.get(key)
    }

    pub fn version(&self, uri: &Uri) -> Option<i32> {
        self.open
            .get(&Self::key(uri))
            .map(|document| document.version)
    }

    pub fn keys(&self) -> Vec<String> {
        self.open.keys().cloned().collect()
    }
}

pub fn uri_to_path(uri: &Uri) -> Option<String> {
    let raw = uri.as_str();
    let rest = raw.strip_prefix("file://")?;
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    let decoded = percent_decode(path);

    Some(trim_drive_prefix(decoded))
}

fn trim_drive_prefix(path: String) -> String {
    let mut characters = path.chars();
    let leading_slash = characters.next() == Some('/');
    let drive = matches!(characters.next(), Some(letter) if letter.is_ascii_alphabetic());

    if leading_slash && drive && characters.next() == Some(':') {
        path.trim_start_matches('/').to_string()
    } else {
        path
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2]))
        {
            decoded.push(high * 16 + low);
            index += 3;
            continue;
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
