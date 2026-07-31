//! Bridge behavior tests against a minimal in-process mock relay: the
//! dominated-write surface, the 429 retry-with-hint path, and rejection.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use buzz_provision::bridge::{self, BridgeError};
use nostr::{EventBuilder, Keys, Kind};

/// Serve `responses` (status line + JSON body) one per request, then stop.
/// Returns (http_base, hit_counter).
fn mock_relay(responses: Vec<(u16, &'static str)>) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_thread = Arc::clone(&hits);
    std::thread::spawn(move || {
        for (status, body) in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            hits_thread.fetch_add(1, Ordering::SeqCst);
            // Drain request head + declared body.
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut content_len = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                let lower = line.to_ascii_lowercase();
                if let Some(v) = lower.strip_prefix("content-length:") {
                    content_len = v.trim().parse().unwrap_or(0);
                }
                if line == "\r\n" {
                    break;
                }
            }
            let mut body_buf = vec![0u8; content_len];
            let _ = reader.read_exact(&mut body_buf);
            let reason = match status {
                200 => "OK",
                429 => "Too Many Requests",
                _ => "Error",
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (base, hits)
}

fn signed_test_event(keys: &Keys) -> nostr::Event {
    EventBuilder::new(Kind::Custom(10100), "{}")
        .sign_with_keys(keys)
        .unwrap()
}

#[test]
fn dominated_replaceable_write_is_an_error_not_success() {
    let keys = Keys::generate();
    let (base, _) = mock_relay(vec![(
        200,
        r#"{"event_id":"aa","accepted":true,"message":"duplicate: replaceable event dominated"}"#,
    )]);
    let client = bridge::client().unwrap();
    let err = bridge::post_event(&client, &base, &keys, None, &signed_test_event(&keys))
        .expect_err("accepted-but-duplicate must NOT be success");
    assert!(matches!(err, BridgeError::Dominated { .. }), "got: {err}");
    assert!(err.to_string().contains("did NOT change"), "got: {err}");
}

#[test]
fn rejected_event_surfaces_relay_message() {
    let keys = Keys::generate();
    let (base, _) = mock_relay(vec![(
        200,
        r#"{"accepted":false,"message":"not an active member"}"#,
    )]);
    let client = bridge::client().unwrap();
    let err = bridge::post_event(&client, &base, &keys, None, &signed_test_event(&keys))
        .expect_err("accepted:false must be an error");
    assert!(
        matches!(&err, BridgeError::Rejected { message } if message.contains("not an active member")),
        "got: {err}"
    );
}

#[test]
fn rate_limited_request_retries_with_hint_and_succeeds() {
    let keys = Keys::generate();
    let (base, hits) = mock_relay(vec![
        (
            429,
            r#"{"error":"rate limited: quota exceeded; retry in 1s"}"#,
        ),
        (
            200,
            r#"{"event_id":"bb","accepted":true,"message":"stored"}"#,
        ),
    ]);
    let client = bridge::client().unwrap();
    let started = std::time::Instant::now();
    let id = bridge::post_event(&client, &base, &keys, None, &signed_test_event(&keys))
        .expect("must succeed after the 429 retry");
    assert_eq!(id, "bb");
    assert_eq!(hits.load(Ordering::SeqCst), 2, "exactly one retry");
    assert!(
        started.elapsed() >= std::time::Duration::from_secs(1),
        "must honor the relay's retry-in hint"
    );
}
