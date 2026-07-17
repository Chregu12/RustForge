//! Independent, adversarial conformance tests for `rf-ai`.
//!
//! These validate the crate against the Anthropic Messages API wire format
//! without assuming the in-crate tests are sufficient. They cover:
//! serialization, deserialization, the agent tool-calling loop, the
//! AnthropicProvider request construction (against a local oneshot TCP stub —
//! never the real API), and the mock embedding provider.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rf_ai::mock::{text_response, MockChatProvider, MockEmbeddingProvider};
use rf_ai::prelude::*;
use rf_ai::response::{ChatResponse, Usage};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// 1. Serialization — exact Anthropic request shape
// ---------------------------------------------------------------------------

/// Helper: collect the top-level object keys of a serialized value.
fn keys(v: &Value) -> Vec<String> {
    v.as_object()
        .expect("expected JSON object")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn minimal_request_omits_optional_keys_entirely() {
    let request = ChatRequest::default_model()
        .max_tokens(256)
        .message(Message::user("hi"));
    let v = serde_json::to_value(&request).unwrap();

    // Required keys present with correct values.
    assert_eq!(v["model"], "claude-opus-4-8");
    assert_eq!(v["max_tokens"], 256);
    assert!(v["messages"].is_array());

    // Optional/empty keys must be ABSENT (not null, not empty).
    let k = keys(&v);
    assert!(!k.contains(&"system".to_string()), "system leaked: {k:?}");
    assert!(!k.contains(&"tools".to_string()), "tools leaked: {k:?}");
    assert!(
        !k.contains(&"tool_choice".to_string()),
        "tool_choice leaked: {k:?}"
    );

    // The exact key set must be only these three.
    let mut got = k.clone();
    got.sort();
    assert_eq!(
        got,
        vec![
            "max_tokens".to_string(),
            "messages".to_string(),
            "model".to_string()
        ]
    );
}

#[test]
fn no_sampling_params_ever_emitted() {
    // Build the most fully-populated request the API supports and confirm
    // temperature/top_p/top_k never appear anywhere in the serialized JSON.
    let request = ChatRequest::default_model()
        .max_tokens(1024)
        .system("sys")
        .tool(Tool::new("t", "d", json!({"type":"object"})))
        .tool_choice(ToolChoice::Any)
        .message(Message::user("u"))
        .message(Message::assistant("a"));
    let text = serde_json::to_string(&request).unwrap();
    for banned in ["temperature", "top_p", "top_k"] {
        assert!(
            !text.contains(banned),
            "sampling param `{banned}` leaked into request: {text}"
        );
    }
}

#[test]
fn empty_tools_vec_is_omitted_but_nonempty_is_present() {
    // Explicitly empty tools -> key absent.
    let req = ChatRequest::default_model().tools(Vec::<Tool>::new());
    let v = serde_json::to_value(&req).unwrap();
    assert!(!keys(&v).contains(&"tools".to_string()));

    // One tool -> key present and well-formed.
    let req = ChatRequest::default_model().tool(Tool::new(
        "get_weather",
        "Get weather",
        json!({"type":"object","properties":{"city":{"type":"string"}},"required":["city"]}),
    ));
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["tools"][0]["name"], "get_weather");
    assert_eq!(v["tools"][0]["description"], "Get weather");
    assert_eq!(v["tools"][0]["input_schema"]["type"], "object");
    // The tool object carries exactly name/description/input_schema.
    let mut tk = keys(&v["tools"][0]);
    tk.sort();
    assert_eq!(
        tk,
        vec![
            "description".to_string(),
            "input_schema".to_string(),
            "name".to_string()
        ]
    );
}

#[test]
fn tool_choice_all_three_shapes() {
    assert_eq!(
        serde_json::to_value(ToolChoice::Auto).unwrap(),
        json!({"type":"auto"})
    );
    assert_eq!(
        serde_json::to_value(ToolChoice::Any).unwrap(),
        json!({"type":"any"})
    );
    assert_eq!(
        serde_json::to_value(ToolChoice::Tool("calculator".into())).unwrap(),
        json!({"type":"tool","name":"calculator"})
    );
}

#[test]
fn message_content_is_always_an_array_of_typed_blocks() {
    // Even a simple user text message serializes content as [ {type:text,...} ].
    let v = serde_json::to_value(Message::user("hello")).unwrap();
    assert_eq!(v["role"], "user");
    assert!(v["content"].is_array(), "content must be an array");
    assert_eq!(v["content"][0]["type"], "text");
    assert_eq!(v["content"][0]["text"], "hello");
}

#[test]
fn content_block_tags_match_anthropic_exactly() {
    // text
    assert_eq!(
        serde_json::to_value(ContentBlock::text("x")).unwrap(),
        json!({"type":"text","text":"x"})
    );
    // tool_use (snake_case tag)
    let tu = ContentBlock::ToolUse {
        id: "tu_1".into(),
        name: "add".into(),
        input: json!({"a":1}),
    };
    assert_eq!(
        serde_json::to_value(&tu).unwrap(),
        json!({"type":"tool_use","id":"tu_1","name":"add","input":{"a":1}})
    );
    // tool_result with is_error
    let tr = ContentBlock::tool_error("tu_1", "boom");
    assert_eq!(
        serde_json::to_value(&tr).unwrap(),
        json!({"type":"tool_result","tool_use_id":"tu_1","content":"boom","is_error":true})
    );
    // tool_result default (non-error) still emits the tool_use_id field correctly
    let tr_ok = ContentBlock::tool_result("tu_2", "42");
    let v = serde_json::to_value(&tr_ok).unwrap();
    assert_eq!(v["type"], "tool_result");
    assert_eq!(v["tool_use_id"], "tu_2");
    assert_eq!(v["content"], "42");
}

#[test]
fn content_block_round_trips_through_json() {
    for block in [
        ContentBlock::text("round"),
        ContentBlock::ToolUse {
            id: "id".into(),
            name: "n".into(),
            input: json!({"k":[1,2,3]}),
        },
        ContentBlock::tool_result("tid", "out"),
        ContentBlock::tool_error("tid", "err"),
    ] {
        let s = serde_json::to_string(&block).unwrap();
        let back: ContentBlock = serde_json::from_str(&s).unwrap();
        assert_eq!(block, back, "round-trip mismatch for {s}");
    }
}

// ---------------------------------------------------------------------------
// 2. Deserialization — realistic Anthropic response
// ---------------------------------------------------------------------------

#[test]
fn realistic_response_with_text_and_tool_use_parses() {
    let payload = json!({
        "id": "msg_01XYZ",
        "type": "message",
        "role": "assistant",
        "model": "claude-opus-4-8",
        "stop_reason": "tool_use",
        "stop_sequence": null,
        "content": [
            {"type":"text","text":"Let me look that up. "},
            {"type":"text","text":"One moment."},
            {"type":"tool_use","id":"toolu_01","name":"get_weather","input":{"city":"Paris","unit":"c"}}
        ],
        "usage": {"input_tokens": 42, "output_tokens": 17}
    });
    let resp: ChatResponse = serde_json::from_value(payload).unwrap();

    // .text() concatenates ONLY text blocks (and concatenates all of them).
    assert_eq!(resp.text(), "Let me look that up. One moment.");
    // .tool_uses() returns just the tool_use block(s).
    let uses = resp.tool_uses();
    assert_eq!(uses.len(), 1);
    match uses[0] {
        ContentBlock::ToolUse { id, name, input } => {
            assert_eq!(id, "toolu_01");
            assert_eq!(name, "get_weather");
            assert_eq!(input["city"], "Paris");
            assert_eq!(input["unit"], "c");
        }
        _ => panic!("expected tool_use"),
    }
    // stop reason / usage.
    assert!(resp.stopped_for_tools());
    assert_eq!(resp.usage.input_tokens, 42);
    assert_eq!(resp.usage.output_tokens, 17);
    assert_eq!(resp.id, "msg_01XYZ");
    assert_eq!(resp.model, "claude-opus-4-8");
    assert_eq!(resp.role, Role::Assistant);
}

#[test]
fn stopped_for_tools_is_true_iff_stop_reason_is_tool_use() {
    let mk = |reason: Option<&str>| ChatResponse {
        id: String::new(),
        model: String::new(),
        role: Role::Assistant,
        stop_reason: reason.map(|s| s.to_string()),
        content: vec![],
        usage: Usage::default(),
    };
    assert!(mk(Some("tool_use")).stopped_for_tools());
    assert!(!mk(Some("end_turn")).stopped_for_tools());
    assert!(!mk(Some("max_tokens")).stopped_for_tools());
    assert!(!mk(None).stopped_for_tools());
}

#[test]
fn response_tolerates_extra_unknown_fields() {
    // Anthropic may add fields; deserialization must not choke.
    let payload = json!({
        "id": "msg_1",
        "type": "message",
        "role": "assistant",
        "model": "claude-opus-4-8",
        "stop_reason": "end_turn",
        "container": {"id": "c1"},
        "some_future_field": [1,2,3],
        "content": [{"type":"text","text":"ok"}],
        "usage": {"input_tokens": 1, "output_tokens": 2, "cache_read_input_tokens": 9}
    });
    let resp: ChatResponse = serde_json::from_value(payload).unwrap();
    assert_eq!(resp.text(), "ok");
    assert_eq!(resp.usage.input_tokens, 1);
    assert_eq!(resp.usage.output_tokens, 2);
}

#[test]
fn response_tolerates_missing_optional_fields() {
    // Only content present; everything else defaulted.
    let payload = json!({"content":[{"type":"text","text":"hi"}]});
    let resp: ChatResponse = serde_json::from_value(payload).unwrap();
    assert_eq!(resp.text(), "hi");
    assert_eq!(resp.role, Role::Assistant);
    assert_eq!(resp.id, "");
    assert_eq!(resp.usage.input_tokens, 0);
    assert!(!resp.stopped_for_tools());

    // Entirely empty object — still parses, empty text.
    let empty: ChatResponse = serde_json::from_value(json!({})).unwrap();
    assert_eq!(empty.text(), "");
    assert_eq!(empty.tool_uses().len(), 0);
}

#[test]
fn multiple_tool_uses_are_all_returned() {
    let payload = json!({
        "content": [
            {"type":"tool_use","id":"a","name":"f","input":{}},
            {"type":"text","text":"between"},
            {"type":"tool_use","id":"b","name":"g","input":{}}
        ]
    });
    let resp: ChatResponse = serde_json::from_value(payload).unwrap();
    assert_eq!(resp.tool_uses().len(), 2);
    assert_eq!(resp.text(), "between");
}

// ---------------------------------------------------------------------------
// 3. Agent tool-calling loop
// ---------------------------------------------------------------------------

fn tool_use_response(id: &str, name: &str, input: Value) -> ChatResponse {
    ChatResponse {
        id: "r".into(),
        model: "mock".into(),
        role: Role::Assistant,
        stop_reason: Some("tool_use".into()),
        content: vec![ContentBlock::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
        }],
        usage: Usage::default(),
    }
}

/// A MockChatProvider wrapper that records every request it receives so we can
/// inspect the conversation the agent built.
struct RecordingProvider {
    inner: MockChatProvider,
    seen: Arc<std::sync::Mutex<Vec<ChatRequest>>>,
}

#[async_trait::async_trait]
impl ChatProvider for RecordingProvider {
    async fn chat(&self, request: &ChatRequest) -> AiResult<ChatResponse> {
        self.seen.lock().unwrap().push(request.clone());
        self.inner.chat(request).await
    }
}

#[tokio::test]
async fn agent_full_loop_appends_assistant_and_tool_result_turns() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let provider = RecordingProvider {
        inner: MockChatProvider::new(vec![
            tool_use_response("toolu_42", "add", json!({"a": 2, "b": 3})),
            text_response("The answer is 5."),
        ]),
        seen: seen.clone(),
    };

    let captured_input = Arc::new(std::sync::Mutex::new(None));
    let ci = captured_input.clone();

    let agent = Agent::new(provider).tool(
        Tool::new("add", "Add two integers", json!({"type":"object"})),
        move |input| {
            *ci.lock().unwrap() = Some(input.clone());
            let a = input["a"].as_i64().unwrap();
            let b = input["b"].as_i64().unwrap();
            Ok((a + b).to_string())
        },
    );

    let answer = agent.run("What is 2 + 3?").await.unwrap();
    assert_eq!(answer, "The answer is 5.");

    // Handler received the correctly-parsed input.
    let got = captured_input.lock().unwrap().clone().unwrap();
    assert_eq!(got, json!({"a": 2, "b": 3}));

    // The SECOND request the agent sent must contain: original user msg,
    // the assistant tool_use turn, then a user turn with a matching tool_result.
    let requests = seen.lock().unwrap();
    assert_eq!(requests.len(), 2, "expected two provider round-trips");
    let msgs = &requests[1].messages;
    assert_eq!(msgs.len(), 3, "user, assistant(tool_use), user(tool_result)");

    assert_eq!(msgs[0].role, Role::User);

    assert_eq!(msgs[1].role, Role::Assistant);
    match &msgs[1].content[0] {
        ContentBlock::ToolUse { id, .. } => assert_eq!(id, "toolu_42"),
        other => panic!("expected assistant tool_use, got {other:?}"),
    }

    assert_eq!(msgs[2].role, Role::User);
    match &msgs[2].content[0] {
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => {
            assert_eq!(tool_use_id, "toolu_42", "tool_use_id must match the call");
            assert_eq!(content, "5");
            assert!(!is_error);
        }
        other => panic!("expected user tool_result, got {other:?}"),
    }
}

#[tokio::test]
async fn agent_unregistered_tool_yields_missing_tool() {
    let provider = MockChatProvider::new(vec![tool_use_response("t", "nonexistent", json!({}))]);
    let agent = Agent::new(provider); // no tools registered
    let err = agent.run("go").await.unwrap_err();
    match err {
        AiError::MissingTool(name) => assert_eq!(name, "nonexistent"),
        other => panic!("expected MissingTool, got {other:?}"),
    }
}

#[tokio::test]
async fn agent_handler_error_becomes_is_error_tool_result() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let provider = RecordingProvider {
        inner: MockChatProvider::new(vec![
            tool_use_response("toolu_err", "boom", json!({})),
            text_response("recovered"),
        ]),
        seen: seen.clone(),
    };
    let agent = Agent::new(provider).tool(
        Tool::new("boom", "always fails", json!({"type":"object"})),
        |_| Err(AiError::MissingTool("simulated failure".into())),
    );

    let answer = agent.run("trigger").await.unwrap();
    assert_eq!(answer, "recovered");

    // The follow-up request must carry a tool_result with is_error = true.
    let requests = seen.lock().unwrap();
    let last_user = requests[1]
        .messages
        .iter()
        .rev()
        .find(|m| m.role == Role::User)
        .unwrap();
    match &last_user.content[0] {
        ContentBlock::ToolResult {
            tool_use_id,
            is_error,
            content,
        } => {
            assert_eq!(tool_use_id, "toolu_err");
            assert!(*is_error, "handler error must surface as is_error tool_result");
            assert!(!content.is_empty(), "error message should be carried");
        }
        other => panic!("expected error tool_result, got {other:?}"),
    }
}

#[tokio::test]
async fn agent_exceeding_max_turns_yields_max_turns_error() {
    // Provider always asks for a tool — agent can never terminate.
    let provider = MockChatProvider::new(vec![tool_use_response("t", "noop", json!({}))]);
    let agent = Agent::new(provider).max_turns(4).tool(
        Tool::new("noop", "no-op", json!({"type":"object"})),
        |_| Ok("ok".into()),
    );
    let err = agent.run("loop").await.unwrap_err();
    match err {
        AiError::MaxTurns(n) => assert_eq!(n, 4),
        other => panic!("expected MaxTurns(4), got {other:?}"),
    }
}

#[tokio::test]
async fn agent_terminates_immediately_on_end_turn() {
    let provider = MockChatProvider::new(vec![text_response("done")]);
    let agent = Agent::new(provider);
    assert_eq!(agent.run("hi").await.unwrap(), "done");
}

#[tokio::test]
async fn agent_handles_multiple_tool_calls_in_one_turn() {
    // Assistant requests two tools in a single turn; agent must dispatch both
    // and produce two matching tool_results in one user turn.
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let two_calls = ChatResponse {
        id: "r".into(),
        model: "mock".into(),
        role: Role::Assistant,
        stop_reason: Some("tool_use".into()),
        content: vec![
            ContentBlock::ToolUse {
                id: "call_a".into(),
                name: "echo".into(),
                input: json!({"v": "x"}),
            },
            ContentBlock::ToolUse {
                id: "call_b".into(),
                name: "echo".into(),
                input: json!({"v": "y"}),
            },
        ],
        usage: Usage::default(),
    };
    let provider = RecordingProvider {
        inner: MockChatProvider::new(vec![two_calls, text_response("both done")]),
        seen: seen.clone(),
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let calls2 = calls.clone();
    let agent = Agent::new(provider).tool(
        Tool::new("echo", "echo", json!({"type":"object"})),
        move |input| {
            calls2.fetch_add(1, Ordering::SeqCst);
            Ok(input["v"].as_str().unwrap_or("").to_string())
        },
    );

    assert_eq!(agent.run("go").await.unwrap(), "both done");
    assert_eq!(calls.load(Ordering::SeqCst), 2, "both handlers invoked");

    let requests = seen.lock().unwrap();
    let results_turn = &requests[1].messages[2];
    assert_eq!(results_turn.role, Role::User);
    assert_eq!(results_turn.content.len(), 2, "one result per tool_use");
    let ids: Vec<&str> = results_turn
        .content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::ToolResult { tool_use_id, .. } => Some(tool_use_id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(ids, vec!["call_a", "call_b"]);
}

// ---------------------------------------------------------------------------
// 4. AnthropicProvider request building (local oneshot stub; NEVER real API)
// ---------------------------------------------------------------------------

/// Bind a TCP listener, accept exactly one connection, read the HTTP request,
/// return a canned 200 response, and hand the raw request bytes back.
async fn oneshot_capture(canned_body: &'static str) -> (String, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");

    let handle = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 8192];
        // Read until we have headers + body. The request is small; one read is
        // typically enough, but loop until we see the end of the body.
        let mut data = Vec::new();
        loop {
            let n = sock.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            // Heuristic: stop once we have the headers and a JSON body that
            // closes its top-level brace.
            let s = String::from_utf8_lossy(&data);
            if s.contains("\r\n\r\n") && s.trim_end().ends_with('}') {
                break;
            }
        }
        let body = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            canned_body.len(),
            canned_body
        );
        sock.write_all(body.as_bytes()).await.unwrap();
        sock.flush().await.unwrap();
        String::from_utf8_lossy(&data).into_owned()
    });

    (base_url, handle)
}

#[tokio::test]
async fn provider_targets_v1_messages_with_required_headers() {
    let canned = r#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-opus-4-8","stop_reason":"end_turn","content":[{"type":"text","text":"hi there"}],"usage":{"input_tokens":3,"output_tokens":2}}"#;
    let (base_url, handle) = oneshot_capture(canned).await;

    let provider = AnthropicProvider::new("test-key-123").with_base_url(&base_url);
    let request = ChatRequest::default_model()
        .max_tokens(64)
        .message(Message::user("ping"));

    let resp = provider.chat(&request).await.unwrap();
    assert_eq!(resp.text(), "hi there");
    assert_eq!(resp.usage.input_tokens, 3);

    let raw = handle.await.unwrap();
    let lower = raw.to_lowercase();

    // Method + path.
    assert!(
        raw.starts_with("POST /v1/messages "),
        "request line wrong: {:?}",
        raw.lines().next()
    );
    // Three required headers (case-insensitive header names).
    assert!(lower.contains("x-api-key: test-key-123"), "missing api key header");
    assert!(
        lower.contains("anthropic-version: 2023-06-01"),
        "missing/incorrect anthropic-version"
    );
    assert!(
        lower.contains("content-type: application/json"),
        "missing content-type"
    );
    // Body is the serialized request.
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("");
    let parsed: Value = serde_json::from_str(body.trim()).unwrap();
    assert_eq!(parsed["model"], "claude-opus-4-8");
    assert_eq!(parsed["max_tokens"], 64);
    assert_eq!(parsed["messages"][0]["content"][0]["text"], "ping");
    // And no sampling params on the wire.
    for banned in ["temperature", "top_p", "top_k"] {
        assert!(!body.contains(banned), "{banned} on the wire");
    }
}

#[tokio::test]
async fn provider_with_base_url_overrides_and_trims_trailing_slash() {
    let canned = r#"{"content":[{"type":"text","text":"ok"}]}"#;
    let (base_url, handle) = oneshot_capture(canned).await;

    // Pass a base_url WITH a trailing slash to exercise trim logic.
    let provider = AnthropicProvider::new("k").with_base_url(format!("{base_url}/"));
    let request = ChatRequest::default_model().message(Message::user("x"));
    let resp = provider.chat(&request).await.unwrap();
    assert_eq!(resp.text(), "ok");

    let raw = handle.await.unwrap();
    // Must be exactly /v1/messages — no double slash.
    assert!(raw.starts_with("POST /v1/messages "), "path: {:?}", raw.lines().next());
    assert!(
        !raw.contains("POST //v1/messages"),
        "trailing slash not trimmed: {:?}",
        raw.lines().next()
    );
}

#[tokio::test]
async fn provider_surfaces_api_error_on_non_2xx() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 4096];
        let _ = sock.read(&mut buf).await.unwrap();
        let err_body = r#"{"type":"error","error":{"type":"invalid_request_error","message":"bad"}}"#;
        let resp = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            err_body.len(),
            err_body
        );
        sock.write_all(resp.as_bytes()).await.unwrap();
        sock.flush().await.unwrap();
    });

    let provider = AnthropicProvider::new("k").with_base_url(&base_url);
    let request = ChatRequest::default_model().message(Message::user("x"));
    let err = provider.chat(&request).await.unwrap_err();
    handle.await.unwrap();
    match err {
        AiError::Api { status, body } => {
            assert_eq!(status, 400);
            assert!(body.contains("invalid_request_error"), "body: {body}");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 5. MockEmbeddingProvider
// ---------------------------------------------------------------------------

#[tokio::test]
async fn embeddings_have_configured_dimension() {
    for dim in [1usize, 4, 16, 128] {
        let provider = MockEmbeddingProvider::new(dim);
        let out = provider.embed(&["a".into(), "bb".into(), "ccc".into()]).await.unwrap();
        assert_eq!(out.len(), 3);
        for v in &out {
            assert_eq!(v.len(), dim, "dim mismatch for {dim}");
        }
    }
}

#[tokio::test]
async fn embeddings_are_deterministic_across_calls() {
    let provider = MockEmbeddingProvider::new(32);
    let texts: Vec<String> = vec!["foo".into(), "bar baz".into()];
    let a = provider.embed(&texts).await.unwrap();
    let b = provider.embed(&texts).await.unwrap();
    assert_eq!(a, b, "same input must yield identical vectors");

    // A fresh provider instance of the same dim must agree too.
    let provider2 = MockEmbeddingProvider::new(32);
    let c = provider2.embed(&texts).await.unwrap();
    assert_eq!(a, c, "determinism must not depend on instance");
}

#[tokio::test]
async fn embeddings_differ_for_different_inputs() {
    let provider = MockEmbeddingProvider::new(64);
    let out = provider
        .embed(&["alpha".into(), "beta".into(), "gamma".into()])
        .await
        .unwrap();
    assert_ne!(out[0], out[1]);
    assert_ne!(out[1], out[2]);
    assert_ne!(out[0], out[2]);
}

#[tokio::test]
async fn embedding_values_are_in_unit_range() {
    let provider = MockEmbeddingProvider::new(50);
    let out = provider.embed(&["some text here".into()]).await.unwrap();
    for &x in &out[0] {
        assert!((-1.0..=1.0).contains(&x), "value {x} outside [-1,1]");
    }
}
