//! Unit tests for `crate::transport`: frame reading, message decoding, and replies.

use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};

use lsp_server::{ErrorCode, Message};
use serde_json::{Value, json};

use super::{Decoded, MAX_FRAME_BYTES, connect, decode, read_frame};

fn frame(body: &str) -> Vec<u8> {
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
}

fn error_code(decoded: Decoded) -> (Value, i64) {
    let Decoded::Reply(reply) = decoded else {
        panic!("expected an error reply, got {decoded:?}");
    };
    (
        reply["id"].clone(),
        reply["error"]["code"].as_i64().expect("an error code"),
    )
}

#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn reads_consecutive_frames() {
    let mut input = Cursor::new([frame("{}"), frame("[1]")].concat());

    assert_eq!(read_frame(&mut input).unwrap(), Some(b"{}".to_vec()));
    assert_eq!(read_frame(&mut input).unwrap(), Some(b"[1]".to_vec()));
    assert_eq!(read_frame(&mut input).unwrap(), None);
}

#[test]
fn refuses_a_frame_above_the_limit_before_reading_it() {
    let header = format!("Content-Length: {}\r\n\r\n", MAX_FRAME_BYTES + 1);
    let mut input = Cursor::new(header.into_bytes());

    let error = read_frame(&mut input).unwrap_err();

    assert!(error.to_string().contains("exceeds the limit"));
}

#[test]
fn refuses_an_overflowing_or_missing_length() {
    for header in [
        "Content-Length: 18446744073709551616\r\n\r\n",
        "Content-Length: -1\r\n\r\n",
        "Content-Type: x\r\n\r\n",
        "Content-Length: 2\n\n{}",
    ] {
        assert!(
            read_frame(&mut Cursor::new(header.as_bytes().to_vec())).is_err(),
            "{header:?}"
        );
    }
}

#[test]
fn a_truncated_frame_is_an_error() {
    let mut input = Cursor::new(b"Content-Length: 10\r\n\r\n{}".to_vec());

    assert!(read_frame(&mut input).is_err());
}

#[test]
fn a_body_that_is_not_json_gets_a_parse_error() {
    assert_eq!(
        error_code(decode(b"{not json")),
        (Value::Null, ErrorCode::ParseError as i64)
    );
    assert_eq!(
        error_code(decode(&[0xff, 0xfe])),
        (Value::Null, ErrorCode::ParseError as i64)
    );
}

#[test]
fn a_request_with_an_unusable_id_gets_an_invalid_request_error() {
    for id in [json!(null), json!(1.5), json!(4_294_967_296_u64)] {
        let body = json!({"jsonrpc": "2.0", "id": id, "method": "textDocument/hover"});

        assert_eq!(
            error_code(decode(body.to_string().as_bytes())),
            (id, ErrorCode::InvalidRequest as i64)
        );
    }
}

#[test]
fn json_that_is_not_a_message_gets_an_invalid_request_error() {
    for body in [json!([]), json!({"jsonrpc": "2.0", "method": 1})] {
        assert_eq!(
            error_code(decode(body.to_string().as_bytes())).1,
            ErrorCode::InvalidRequest as i64
        );
    }
}

#[test]
fn requests_and_notifications_decode() {
    let request = json!({"jsonrpc": "2.0", "id": 7, "method": "shutdown"});
    let notification = json!({"jsonrpc": "2.0", "method": "exit"});

    assert!(matches!(
        decode(request.to_string().as_bytes()),
        Decoded::Message(Message::Request(_))
    ));
    assert!(matches!(
        decode(notification.to_string().as_bytes()),
        Decoded::Message(Message::Notification(_))
    ));
}

#[test]
fn a_malformed_message_is_answered_and_reading_continues() {
    let good = json!({"jsonrpc": "2.0", "method": "initialized", "params": {}});
    let input = [frame("{oops"), frame(&good.to_string())].concat();
    let output = Shared::default();
    let (connection, writer) = connect(Cursor::new(input), output.clone()).unwrap();

    let message = connection.receiver.recv().unwrap();
    drop(connection);
    writer.join().unwrap();

    assert!(
        matches!(message, Message::Notification(notification) if notification.method == "initialized")
    );
    let written = String::from_utf8(output.0.lock().unwrap().clone()).unwrap();
    assert!(written.starts_with("Content-Length: "));
    assert!(written.contains("\"code\":-32700"));
}
