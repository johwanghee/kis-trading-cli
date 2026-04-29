use std::io::{self, Write};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::time::{timeout_at, Instant};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone)]
pub struct WsSubscribeRequest {
    pub websocket_url: String,
    pub approval_key: String,
    pub tr_id: String,
    pub tr_key: String,
    pub tr_type: String,
    pub limit: Option<usize>,
    pub duration: Option<Duration>,
    pub compact: bool,
}

#[derive(Debug, Serialize)]
struct WsEvent<'a> {
    #[serde(rename = "type")]
    event_type: &'a str,
    received_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tr_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tr_key: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tr_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_format: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

pub async fn subscribe(request: WsSubscribeRequest) -> Result<()> {
    let url = websocket_endpoint(&request.websocket_url);
    let deadline = request.duration.map(|duration| Instant::now() + duration);
    let (mut socket, _) = connect_async(&url)
        .await
        .with_context(|| format!("failed to connect KIS websocket `{url}`"))?;

    print_event(
        &WsEvent {
            event_type: "connected",
            received_at: now_string(),
            tr_id: None,
            tr_key: None,
            tr_type: None,
            message_format: None,
            raw: None,
            data: Some(json!({ "websocket_url": url })),
        },
        request.compact,
    )?;

    let subscribe_message = json!({
        "header": {
            "approval_key": request.approval_key,
            "custtype": "P",
            "tr_type": request.tr_type,
            "content-type": "utf-8",
        },
        "body": {
            "input": {
                "tr_id": request.tr_id,
                "tr_key": request.tr_key,
            }
        }
    });

    socket
        .send(Message::Text(subscribe_message.to_string().into()))
        .await
        .context("failed to send KIS websocket subscribe message")?;

    print_event(
        &WsEvent {
            event_type: "subscribed",
            received_at: now_string(),
            tr_id: Some(&request.tr_id),
            tr_key: Some(&request.tr_key),
            tr_type: Some(&request.tr_type),
            message_format: None,
            raw: None,
            data: None,
        },
        request.compact,
    )?;

    let mut message_count = 0usize;

    loop {
        if request.limit.is_some_and(|limit| message_count >= limit) {
            break;
        }

        let next_message = match deadline {
            Some(deadline) => match timeout_at(deadline, socket.next()).await {
                Ok(value) => value,
                Err(_) => break,
            },
            None => socket.next().await,
        };

        let Some(next_message) = next_message else {
            break;
        };

        match next_message.context("failed to receive KIS websocket message")? {
            Message::Text(text) => {
                handle_text_message(
                    &mut socket,
                    text.as_ref(),
                    &mut message_count,
                    request.compact,
                )
                .await?;
            }
            Message::Binary(bytes) => {
                message_count += 1;
                print_event(
                    &WsEvent {
                        event_type: "message",
                        received_at: now_string(),
                        tr_id: None,
                        tr_key: None,
                        tr_type: None,
                        message_format: Some("binary"),
                        raw: None,
                        data: Some(json!({ "byte_count": bytes.len() })),
                    },
                    request.compact,
                )?;
            }
            Message::Ping(payload) => {
                socket
                    .send(Message::Pong(payload))
                    .await
                    .context("failed to send KIS websocket pong")?;
                print_event(
                    &WsEvent {
                        event_type: "ping",
                        received_at: now_string(),
                        tr_id: None,
                        tr_key: None,
                        tr_type: None,
                        message_format: None,
                        raw: None,
                        data: None,
                    },
                    request.compact,
                )?;
            }
            Message::Pong(_) => {
                print_event(
                    &WsEvent {
                        event_type: "pong",
                        received_at: now_string(),
                        tr_id: None,
                        tr_key: None,
                        tr_type: None,
                        message_format: None,
                        raw: None,
                        data: None,
                    },
                    request.compact,
                )?;
            }
            Message::Close(frame) => {
                print_event(
                    &WsEvent {
                        event_type: "closed",
                        received_at: now_string(),
                        tr_id: None,
                        tr_key: None,
                        tr_type: None,
                        message_format: None,
                        raw: None,
                        data: Some(json!({ "frame": frame.map(|frame| frame.to_string()) })),
                    },
                    request.compact,
                )?;
                break;
            }
            Message::Frame(_) => {}
        }
    }

    let _ = socket.close(None).await;
    Ok(())
}

async fn handle_text_message<S>(
    socket: &mut S,
    text: &str,
    message_count: &mut usize,
    compact: bool,
) -> Result<()>
where
    S: SinkExt<Message> + Unpin,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    if is_pingpong_text(text) {
        socket
            .send(Message::Pong(text.as_bytes().to_vec().into()))
            .await
            .context("failed to send KIS websocket pong")?;
        print_event(
            &WsEvent {
                event_type: "pingpong",
                received_at: now_string(),
                tr_id: Some("PINGPONG"),
                tr_key: None,
                tr_type: None,
                message_format: Some("json"),
                raw: Some(text),
                data: parse_json(text),
            },
            compact,
        )?;
        return Ok(());
    }

    *message_count += 1;
    let parsed = parse_realtime_text(text);
    print_event(
        &WsEvent {
            event_type: "message",
            received_at: now_string(),
            tr_id: parsed.tr_id.as_deref(),
            tr_key: None,
            tr_type: None,
            message_format: Some(parsed.format),
            raw: Some(text),
            data: parsed.data,
        },
        compact,
    )
}

struct ParsedMessage {
    format: &'static str,
    tr_id: Option<String>,
    data: Option<Value>,
}

fn parse_realtime_text(text: &str) -> ParsedMessage {
    if matches!(text.as_bytes().first(), Some(b'0' | b'1')) {
        let parts: Vec<&str> = text.splitn(4, '|').collect();
        return ParsedMessage {
            format: "pipe",
            tr_id: parts.get(1).map(|value| (*value).to_string()),
            data: Some(json!({
                "prefix": parts.first().copied(),
                "tr_id": parts.get(1).copied(),
                "record_count": parts.get(2).copied(),
                "payload": parts.get(3).copied(),
            })),
        };
    }

    ParsedMessage {
        format: "json",
        tr_id: parse_json(text)
            .as_ref()
            .and_then(|value| value.pointer("/header/tr_id"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        data: parse_json(text),
    }
}

fn is_pingpong_text(text: &str) -> bool {
    parse_json(text)
        .as_ref()
        .and_then(|value| value.pointer("/header/tr_id"))
        .and_then(Value::as_str)
        == Some("PINGPONG")
}

fn parse_json(text: &str) -> Option<Value> {
    serde_json::from_str(text).ok()
}

fn websocket_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/tryitout") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/tryitout")
    }
}

fn now_string() -> String {
    Utc::now().to_rfc3339()
}

fn print_event(event: &WsEvent<'_>, _compact: bool) -> Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, event).context("failed to serialize websocket event")?;
    stdout
        .write_all(b"\n")
        .context("failed to write websocket event")?;
    stdout.flush().context("failed to flush websocket event")?;
    Ok(())
}

pub fn parse_duration(value: &str) -> Result<Duration> {
    let value = value.trim();
    if value.is_empty() {
        bail!("duration cannot be empty");
    }

    let (number, multiplier) = if let Some(number) = value.strip_suffix("ms") {
        (number, 1)
    } else if let Some(number) = value.strip_suffix('s') {
        (number, 1_000)
    } else if let Some(number) = value.strip_suffix('m') {
        (number, 60_000)
    } else if let Some(number) = value.strip_suffix('h') {
        (number, 3_600_000)
    } else {
        (value, 1_000)
    };

    let amount: u64 = number
        .parse()
        .with_context(|| format!("invalid duration `{value}`"))?;
    Ok(Duration::from_millis(amount.saturating_mul(multiplier)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_endpoint_appends_tryitout_once() {
        assert_eq!(
            websocket_endpoint("ws://ops.koreainvestment.com:31000"),
            "ws://ops.koreainvestment.com:31000/tryitout"
        );
        assert_eq!(
            websocket_endpoint("ws://ops.koreainvestment.com:31000/tryitout"),
            "ws://ops.koreainvestment.com:31000/tryitout"
        );
    }

    #[test]
    fn parse_duration_supports_common_units() {
        assert_eq!(parse_duration("5").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
        assert_eq!(parse_duration("250ms").unwrap(), Duration::from_millis(250));
    }
}
