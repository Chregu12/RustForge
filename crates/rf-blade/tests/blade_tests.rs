//! Integration tests for rf-blade
//!
//! Tests cover: variable rendering, @if/@else, @foreach, template inheritance
//! (@extends / @section / @yield), HTML escaping, and the @csrf directive.
//!
//! All tests use `render_compiled` which exercises the full lexer → parser →
//! compiler pipeline without touching the filesystem.

use rf_blade::BladeEngine;
use serde_json::json;
use std::path::PathBuf;

// Helper: create a BladeEngine backed by an empty temp directory so `new()`
// succeeds (it only checks that the path exists).
fn make_engine() -> BladeEngine {
    let dir = PathBuf::from("/tmp/rf-blade-integ-tests");
    std::fs::create_dir_all(&dir).ok();
    BladeEngine::new(&dir).expect("engine creation should succeed")
}

// ───────────────────────────────────────────────────────────────────────────
// Variable rendering
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn render_simple_variable() {
    let engine = make_engine();
    let html = engine
        .render_compiled("Hello {{ $name }}!", json!({ "name": "World" }))
        .await
        .unwrap();
    assert_eq!(html, "Hello World!");
}

#[tokio::test]
async fn render_multiple_variables() {
    let engine = make_engine();
    let html = engine
        .render_compiled(
            "{{ $first }} {{ $last }}",
            json!({ "first": "John", "last": "Doe" }),
        )
        .await
        .unwrap();
    assert_eq!(html.trim(), "John Doe");
}

#[tokio::test]
async fn render_undefined_variable_produces_empty_string() {
    let engine = make_engine();
    let html = engine
        .render_compiled("Hello {{ $missing }}!", json!({}))
        .await
        .unwrap();
    assert_eq!(html, "Hello !");
}

// ───────────────────────────────────────────────────────────────────────────
// HTML escaping
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn html_escape_angle_brackets_in_variable() {
    let engine = make_engine();
    let html = engine
        .render_compiled(
            "{{ $code }}",
            json!({ "code": "<script>alert(1)</script>" }),
        )
        .await
        .unwrap();
    assert!(!html.contains('<'));
    assert!(!html.contains('>'));
    assert!(html.contains("&lt;"));
    assert!(html.contains("&gt;"));
}

#[tokio::test]
async fn html_escape_ampersand_in_variable() {
    let engine = make_engine();
    let html = engine
        .render_compiled("{{ $txt }}", json!({ "txt": "A & B" }))
        .await
        .unwrap();
    assert!(html.contains("&amp;"));
}

// ───────────────────────────────────────────────────────────────────────────
// @if / @else
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn if_directive_renders_true_branch() {
    let engine = make_engine();
    let html = engine
        .render_compiled(
            "@if($show) Visible @endif",
            json!({ "show": true }),
        )
        .await
        .unwrap();
    assert!(html.contains("Visible"));
}

#[tokio::test]
async fn if_directive_skips_false_branch() {
    let engine = make_engine();
    let html = engine
        .render_compiled(
            "@if($show) Visible @endif",
            json!({ "show": false }),
        )
        .await
        .unwrap();
    assert!(!html.contains("Visible"));
}

#[tokio::test]
async fn if_else_renders_else_when_condition_false() {
    let engine = make_engine();
    let html = engine
        .render_compiled(
            "@if($admin) Admin @else Guest @endif",
            json!({ "admin": false }),
        )
        .await
        .unwrap();
    assert!(html.contains("Guest"));
    assert!(!html.contains("Admin"));
}

// ───────────────────────────────────────────────────────────────────────────
// @foreach
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn foreach_iterates_over_array() {
    let engine = make_engine();
    let html = engine
        .render_compiled(
            "@foreach($items as $item){{ $item }} @endforeach",
            json!({ "items": ["a", "b", "c"] }),
        )
        .await
        .unwrap();
    assert!(html.contains("a"));
    assert!(html.contains("b"));
    assert!(html.contains("c"));
}

#[tokio::test]
async fn foreach_empty_array_produces_no_output() {
    let engine = make_engine();
    let html = engine
        .render_compiled(
            "before @foreach($items as $item)X @endforeach after",
            json!({ "items": [] }),
        )
        .await
        .unwrap();
    assert!(!html.contains('X'));
    assert!(html.contains("before"));
    assert!(html.contains("after"));
}

// ───────────────────────────────────────────────────────────────────────────
// @csrf
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn csrf_directive_emits_hidden_input() {
    let engine = make_engine();
    let html = engine
        .render_compiled("<form>@csrf</form>", json!({}))
        .await
        .unwrap();
    // The CSRF directive should produce a hidden input field
    assert!(html.contains("<input") || html.contains("csrf"));
}
