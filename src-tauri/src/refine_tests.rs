use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
fn long_input_splits_without_losing_text_or_cutting_unicode() {
    let raw = "A short sentence. 日本語の文章です。 Another sentence. ".repeat(30);
    let parts = chunks(&raw, 256).unwrap();
    assert!(parts.len() > 1);
    assert!(parts.iter().all(|s| s.chars().count() <= 256));
    assert_eq!(parts.concat(), raw);
    assert!(chunks(&"x".repeat(300), 256).is_err());
}

#[tokio::test]
async fn a_failed_chunk_discards_all_partial_edits() {
    let raw = "alpha ".repeat(50);
    let (mut settings, task) =
        server(vec![(200, response("Alpha."), 0), (500, "{}".into(), 0)]).await;
    settings.max_input_chars = 256;
    let result = run(&settings, &raw, "Clean.", &[]).await;
    assert_eq!(result.text, raw);
    assert_eq!(result.outcome, "raw_http_500");
    assert_eq!(task.await.unwrap().len(), 2);
}

#[tokio::test]
async fn context_limit_errors_retry_smaller_chunks() {
    let raw = "alpha ".repeat(50);
    let (mut settings, task) = server(vec![
        (
            400,
            json!({"error":{"message":"maximum context length exceeded"}}).to_string(),
            0,
        ),
        (200, response("Alpha."), 0),
        (200, response("Alpha."), 0),
    ])
    .await;
    settings.max_input_chars = 512;
    let result = run(&settings, &raw, "Clean.", &[]).await;
    assert_eq!(result.outcome, "cleaned_chunks");
    assert_eq!(result.text, "Alpha. Alpha.");
    assert_eq!(task.await.unwrap().len(), 3);
}

#[tokio::test]
async fn chunks_share_one_deadline() {
    let raw = "alpha ".repeat(50);
    let (mut settings, task) = server(vec![
        (200, response("Alpha."), 70),
        (200, response("Alpha."), 70),
    ])
    .await;
    settings.max_input_chars = 256;
    settings.timeout_ms = 100;
    let result = run(&settings, &raw, "Clean.", &[]).await;
    assert_eq!(result.text, raw);
    assert_eq!(result.outcome, "raw_timeout");
    assert!(result.duration_ms < 300);
    task.abort();
}

fn response(content: &str) -> String {
    json!({"choices":[{"finish_reason":"stop","message":{"content":content}}]}).to_string()
}

/// A real local HTTP socket, with fixed test responses, not used by the app.
async fn server(
    replies: Vec<(u16, String, u64)>,
) -> (RefineSettings, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body, delay_ms) in replies {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buf = [0; 4096];
                let n = stream.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&bytes);
                if let Some((headers, body)) = text.split_once("\r\n\r\n") {
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|s| s.trim().parse().ok())
                        })
                        .unwrap_or(0);
                    if body.len() >= length {
                        break;
                    }
                }
            }
            requests.push(String::from_utf8(bytes).unwrap());
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            let headers = format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = stream
                .write_all(format!("{headers}{body}").as_bytes())
                .await;
        }
        requests
    });
    (
        RefineSettings {
            base_url: format!("http://{address}/v1"),
            timeout_ms: 1000,
            ..Default::default()
        },
        task,
    )
}

#[test]
fn endpoint_has_one_chat_suffix_and_rejects_credential_urls() {
    for base in [
        "http://localhost:11434/v1",
        "http://127.0.0.1:10531/v1/",
        "https://api.openai.com/v1",
    ] {
        assert_eq!(
            endpoint_url(base).unwrap().as_str(),
            format!("{}/chat/completions", base.trim_end_matches('/'))
        );
    }
    for base in [
        "file:///tmp/a",
        "https://secret@example.com/v1",
        "https://example.com/v1?key=secret",
        "https://example.com/#x",
    ] {
        assert!(endpoint_url(base).is_err());
    }
}

#[test]
fn hygiene_unwraps_preamble_fences_quotes_and_thinking() {
    for content in [
        "Here is the cleaned text: \"Send it today.\"",
        "```text\nSend it today.\n```",
        "<think>an explanation</think>\n‘Send it today.’",
        "\"Here is the cleaned text: ```\nSend it today.\n```\"",
    ] {
        assert_eq!(clean_output(content), "Send it today.");
    }
    assert!(plausibility("send it today", "<think>not finished", &[]).is_err());
}

#[test]
fn guard_rejects_answers_invention_changed_numbers_and_lost_negation() {
    for (raw, output) in [
        (
            "what is the capital of France",
            "The capital of France is Paris.",
        ),
        ("can you send this today", "Yes, I can send this today."),
        (
            "hello",
            "Hello. This is a very long and unrelated explanation.",
        ),
        ("set max_retries to 3", "Set max_retries to 4."),
        ("do not ship this", "Do ship this."),
        ("i can't come", "I can come."),
        ("send it", ""),
    ] {
        assert!(plausibility(raw, output, &[]).is_err(), "accepted {output}");
    }
}

#[test]
fn guard_allows_cleanup_corrections_and_questions() {
    for (raw, output) in [
        (
            "um i i think we should ship today",
            "I think we should ship today.",
        ),
        ("send it Tuesday no Wednesday", "Send it Wednesday."),
        ("can you send this today", "Can you send this today?"),
        ("i can't come", "I can't come."),
    ] {
        assert!(plausibility(raw, output, &[]).is_ok(), "rejected {output}");
    }
}

#[test]
fn guard_rejects_the_abandoned_choice() {
    assert_eq!(
        plausibility("send it Tuesday no Wednesday", "Send it Tuesday.", &[]),
        Err("lost_correction")
    );
    assert_eq!(
        plausibility(
            "ask Maya sorry ask Nora to review it",
            "Ask Maya to review it.",
            &[]
        ),
        Err("lost_correction")
    );
}

#[test]
fn fenced_language_labels_and_literal_identifiers_are_handled() {
    assert!(!literal_tokens_preserved("run getUser", "run getUsers"));
    assert_eq!(clean_output("```python\nprint('hi')\n```"), "print('hi')");
    assert!(!literal_tokens_preserved(
        "run getUser with max_retries",
        "Run getuser with maxRetries."
    ));
    assert!(!literal_tokens_preserved(
        "edit /etc/hosts",
        "Edit /etc/host."
    ));
    assert!(literal_tokens_preserved(
        "um run getUser with --dry-run",
        "Run getUser with --dry-run"
    ));
}

#[test]
fn examples_use_the_real_input_envelope() {
    let prompt = "Edit text.\n\nTranscript: uh hello\nCleaned: Hello.";
    let pairs = messages(prompt, "bye", &[]);
    assert_eq!(pairs.len(), 4);
    assert_eq!(pairs[0]["content"], "Edit text.\n");
    assert_eq!(pairs[1]["content"], "{\"dictation\":\"uh hello\"}");
    assert_eq!(pairs[2]["role"], "assistant");
    assert_eq!(pairs[2]["content"], "Hello.");
}

#[test]
fn replacements_are_literal_and_respect_word_boundaries() {
    let replacements = vec![
        Replacement {
            from: "open claw".into(),
            to: "OpenClaw".into(),
        },
        Replacement {
            from: "price".into(),
            to: "$5".into(),
        },
    ];
    assert_eq!(
        apply_replacements(
            "OPEN CLAW has a price, not a pricetag or open claws.",
            &replacements
        ),
        "OpenClaw has a $5, not a pricetag or open claws."
    );
}

#[tokio::test]
async fn request_uses_configured_endpoint_key_model_and_dictionary() {
    let (mut settings, task) = server(vec![(200, response("Use OpenClaw."), 0)]).await;
    settings.api_key = "test-secret".into();
    settings.model = "user-model".into();
    let result = run(
        &settings,
        "use OpenClaw",
        "Only clean the transcript.",
        &["OpenClaw".into()],
    )
    .await;
    assert_eq!(result.text, "Use OpenClaw.");
    assert_eq!(result.outcome, "cleaned");
    let requests = task.await.unwrap();
    assert!(requests[0].starts_with("POST /v1/chat/completions "));
    assert!(requests[0].contains("authorization: Bearer test-secret"));
    let body: Value = serde_json::from_str(requests[0].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["model"], "user-model");
    let input: Value =
        serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(input["dictation"], "use OpenClaw");
    assert!(body["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains("OpenClaw"));
    assert!(!format!("{settings:?}").contains("test-secret"));
}

#[tokio::test]
async fn endpoint_errors_and_bad_output_return_exact_raw() {
    let raw = "  can you send this today  ";
    for (status, body) in [
        (500, "private server detail".into()),
        (401, "secret key invalid".into()),
        (200, "not json".into()),
        (200, response("Yes, I can send this today.")),
        (200, response("")),
        (
            200,
            json!({"choices":[{"finish_reason":"length","message":{"content":"Can you"}}]})
                .to_string(),
        ),
        (302, "redirect".into()),
    ] {
        let (settings, task) = server(vec![(status, body, 0)]).await;
        let result = run(&settings, raw, "Clean only.", &[]).await;
        assert_eq!(result.text, raw);
        assert!(result.outcome.starts_with("raw_"));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn whole_request_timeout_returns_raw_without_waiting_for_server() {
    let (mut settings, task) = server(vec![(200, response("Hello."), 500)]).await;
    settings.timeout_ms = 100;
    let result = run(&settings, "hello", "Clean only.", &[]).await;
    assert_eq!(result.text, "hello");
    assert_eq!(result.outcome, "raw_timeout");
    assert!(result.duration_ms < 400);
    task.abort();
}

#[tokio::test]
async fn optional_parameter_support_is_negotiated_from_errors() {
    let (settings, task) = server(vec![
        (
            400,
            json!({"error":{"message":"Unsupported reasoning_effort"}}).to_string(),
            0,
        ),
        (
            400,
            json!({"error":{"param":"max_tokens","message":"Use max_completion_tokens"}})
                .to_string(),
            0,
        ),
        (200, response("Hello."), 0),
    ])
    .await;
    let result = run(&settings, "hello", "Clean only.", &[]).await;
    assert_eq!(result.outcome, "cleaned");
    let requests = task.await.unwrap();
    assert!(!requests[1].contains("reasoning_effort"));
    assert!(requests[2].contains("max_completion_tokens"));
    assert!(!requests[2].contains("\"max_tokens\""));
}

#[tokio::test]
async fn empty_and_disabled_inputs_never_contact_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut settings = RefineSettings {
        base_url: format!("http://{}/v1", listener.local_addr().unwrap()),
        ..Default::default()
    };
    assert_eq!(
        run(&settings, " \n\t", "Clean.", &[]).await.outcome,
        "empty"
    );
    settings.enabled = false;
    assert_eq!(run(&settings, "hello", "Clean.", &[]).await.text, "hello");
    assert!(
        tokio::time::timeout(Duration::from_millis(25), listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn connection_refused_returns_raw() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let settings = RefineSettings {
        base_url: format!("http://{}/v1", listener.local_addr().unwrap()),
        ..Default::default()
    };
    drop(listener);
    let result = run(&settings, "hello", "Clean.", &[]).await;
    assert_eq!(result.text, "hello");
    assert_eq!(result.outcome, "raw_connection");
}
