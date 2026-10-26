use std::io::{self, BufRead, Read, Write};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, bounded, never, select, unbounded};
use lsp_server::{Connection, ErrorCode, Message};
use serde_json::{Value, json};

use crate::server::{EXIT, SERVER_NAME};

pub const MAX_FRAME_BYTES: usize = 256 * 1024 * 1024;
const MAX_HEADER_BYTES: u64 = 8 * 1024;
const CONTENT_LENGTH: &str = "content-length";

/// Connects a server to a byte stream pair speaking LSP base-protocol frames.
///
/// The reader answers a body that is not JSON, or not a JSON-RPC message,
/// with an error response and keeps reading; it reports a broken frame on
/// stderr and closes the connection. The returned handle finishes once the
/// server drops its `Connection` and every queued message is written.
pub fn connect<R, W>(input: R, output: W) -> io::Result<(Connection, JoinHandle<()>)>
where
    R: BufRead + Send + 'static,
    W: Write + Send + 'static,
{
    let (incoming_sender, incoming) = bounded::<Message>(0);
    let (outgoing, outgoing_receiver) = bounded::<Message>(0);
    let (reply_sender, replies) = unbounded::<Value>();

    thread::Builder::new()
        .name("LspReader".to_string())
        .spawn(move || read_messages(input, &incoming_sender, &reply_sender))?;
    let writer = thread::Builder::new()
        .name("LspWriter".to_string())
        .spawn(move || write_messages(output, &outgoing_receiver, replies))?;

    Ok((
        Connection {
            sender: outgoing,
            receiver: incoming,
        },
        writer,
    ))
}

fn read_messages(mut input: impl BufRead, messages: &Sender<Message>, replies: &Sender<Value>) {
    loop {
        let body = match read_frame(&mut input) {
            Ok(Some(body)) => body,
            Ok(None) => return,
            Err(error) => {
                eprintln!("{}: cannot read the next message: {}", SERVER_NAME, error);
                return;
            }
        };

        match decode(&body) {
            Decoded::Message(message) => {
                let exit = matches!(&message, Message::Notification(notification) if notification.method == EXIT);
                if messages.send(message).is_err() || exit {
                    return;
                }
            }
            Decoded::Reply(reply) => {
                if replies.send(reply).is_err() {
                    return;
                }
            }
        }
    }
}

/// Reads one frame body. Returns `None` at the end of the input before a
/// frame starts.
pub fn read_frame(input: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut length = None;
    let mut started = false;
    let mut line = Vec::new();

    loop {
        line.clear();
        input
            .by_ref()
            .take(MAX_HEADER_BYTES)
            .read_until(b'\n', &mut line)?;
        if line.is_empty() {
            return if started {
                Err(invalid("the input ended inside a header"))
            } else {
                Ok(None)
            };
        }
        started = true;

        let header = line
            .strip_suffix(b"\r\n")
            .ok_or_else(|| invalid("a header line does not end with CRLF"))?;
        if header.is_empty() {
            break;
        }
        let header = std::str::from_utf8(header).map_err(|_| invalid("a header is not UTF-8"))?;
        let (name, value) = header
            .split_once(": ")
            .ok_or_else(|| invalid("a header has no value"))?;
        if name.eq_ignore_ascii_case(CONTENT_LENGTH) {
            let declared: u64 = value
                .trim()
                .parse()
                .map_err(|_| invalid("Content-Length is not a number"))?;
            if declared > MAX_FRAME_BYTES as u64 {
                return Err(invalid(&format!(
                    "a frame of {} bytes exceeds the limit of {} bytes",
                    declared, MAX_FRAME_BYTES
                )));
            }
            length = Some(declared as usize);
        }
    }

    let length = length.ok_or_else(|| invalid("a frame has no Content-Length"))?;
    let mut body = Vec::new();
    input.by_ref().take(length as u64).read_to_end(&mut body)?;
    if body.len() < length {
        return Err(invalid("the input ended inside a frame"));
    }

    Ok(Some(body))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

#[derive(Debug)]
pub enum Decoded {
    Message(Message),
    Reply(Value),
}

/// Decodes a frame body into a message, or into the error response that
/// JSON-RPC prescribes for it: a parse error for a body that is not JSON,
/// and an invalid request for anything else that is not a message,
/// including a request whose id is neither an integer nor a string.
pub fn decode(body: &[u8]) -> Decoded {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return Decoded::Reply(error_reply(
            Value::Null,
            ErrorCode::ParseError,
            "the message is not valid JSON",
        ));
    };
    let id = value.get("id").cloned();

    match serde_json::from_value::<Message>(value) {
        Ok(Message::Notification(_)) if id.is_some() => Decoded::Reply(error_reply(
            id.unwrap_or(Value::Null),
            ErrorCode::InvalidRequest,
            "a request id must be an integer or a string",
        )),
        Ok(message) => Decoded::Message(message),
        Err(_) => Decoded::Reply(error_reply(
            id.unwrap_or(Value::Null),
            ErrorCode::InvalidRequest,
            "the message is not a JSON-RPC request, response, or notification",
        )),
    }
}

fn error_reply(id: Value, code: ErrorCode, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code as i32, "message": message},
    })
}

fn write_messages(mut output: impl Write, messages: &Receiver<Message>, replies: Receiver<Value>) {
    let mut replies = replies;
    loop {
        select! {
            recv(messages) -> message => match message {
                Ok(message) => {
                    if message.write(&mut output).is_err() {
                        return;
                    }
                }
                Err(_) => {
                    for reply in replies.try_iter() {
                        if write_value(&mut output, &reply).is_err() {
                            return;
                        }
                    }
                    return;
                }
            },
            recv(replies) -> reply => match reply {
                Ok(reply) => {
                    if write_value(&mut output, &reply).is_err() {
                        return;
                    }
                }
                Err(_) => replies = never(),
            },
        }
    }
}

fn write_value(output: &mut impl Write, value: &Value) -> io::Result<()> {
    let text = serde_json::to_string(value)?;
    write!(output, "Content-Length: {}\r\n\r\n{}", text.len(), text)?;
    output.flush()
}

#[cfg(test)]
mod tests;
