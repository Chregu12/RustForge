//! Output-assertion tests for the Laravel 13 Blade directives implemented in
//! the `_new` pipeline (lexer -> parser_new -> compiler_new).
//!
//! Each test renders a template against some JSON data and asserts the EXACT
//! resulting string.

use rf_blade::parser_new::Parser;
use rf_blade::{Compiler, RenderContext};
use serde_json::{json, Value};

/// Helper: parse + compile a template with the given data, returning output.
fn render(template: &str, data: Value) -> String {
    let nodes = Parser::parse(template).expect("parse failed");
    let mut ctx = RenderContext::new(data);
    let compiler = Compiler::new();
    compiler.compile(&nodes, &mut ctx).expect("compile failed")
}

// ---------------------------------------------------------------------------
// 1. @unless
// ---------------------------------------------------------------------------

#[test]
fn unless_renders_when_falsy() {
    // cond falsy -> body renders
    assert_eq!(
        render("@unless($admin)guest@endunless", json!({ "admin": false })),
        "guest"
    );
    // cond truthy -> nothing
    assert_eq!(
        render("@unless($admin)guest@endunless", json!({ "admin": true })),
        ""
    );
    // missing var is falsy -> renders
    assert_eq!(render("@unless($admin)guest@endunless", json!({})), "guest");
}

// ---------------------------------------------------------------------------
// 2. @isset
// ---------------------------------------------------------------------------

#[test]
fn isset_present_and_nonnull() {
    assert_eq!(
        render("@isset($name)yes@endisset", json!({ "name": "Alice" })),
        "yes"
    );
    // present but null -> not set
    assert_eq!(
        render("@isset($name)yes@endisset", json!({ "name": null })),
        ""
    );
    // missing -> not set
    assert_eq!(render("@isset($name)yes@endisset", json!({})), "");
    // present-but-empty-string still counts as set
    assert_eq!(
        render("@isset($name)yes@endisset", json!({ "name": "" })),
        "yes"
    );
}

// ---------------------------------------------------------------------------
// 3. @empty
// ---------------------------------------------------------------------------

#[test]
fn empty_block() {
    assert_eq!(
        render("@empty($items)none@endempty", json!({ "items": [] })),
        "none"
    );
    assert_eq!(
        render("@empty($items)none@endempty", json!({ "items": ["a"] })),
        ""
    );
    // missing / null / 0 / "" are empty
    assert_eq!(render("@empty($x)e@endempty", json!({})), "e");
    assert_eq!(render("@empty($x)e@endempty", json!({ "x": null })), "e");
    assert_eq!(render("@empty($x)e@endempty", json!({ "x": 0 })), "e");
    assert_eq!(render("@empty($x)e@endempty", json!({ "x": "" })), "e");
    // non-empty
    assert_eq!(render("@empty($x)e@endempty", json!({ "x": "hi" })), "");
}

// ---------------------------------------------------------------------------
// 4. @switch
// ---------------------------------------------------------------------------

#[test]
fn switch_matches_case() {
    let tpl = "@switch($n)@case(1)A@break@case(2)B@break@default C@endswitch";
    assert_eq!(render(tpl, json!({ "n": 2 })), "B");
    assert_eq!(render(tpl, json!({ "n": 1 })), "A");
    // no match -> default (note leading space before C in template)
    assert_eq!(render(tpl, json!({ "n": 9 })), " C");
}

#[test]
fn switch_no_default_no_match() {
    let tpl = "@switch($n)@case(1)A@break@endswitch";
    assert_eq!(render(tpl, json!({ "n": 5 })), "");
}

// ---------------------------------------------------------------------------
// 5. @forelse
// ---------------------------------------------------------------------------

#[test]
fn forelse_empty_and_nonempty() {
    let tpl = "@forelse($xs as $x){{ $x }}@empty none@endforelse";
    assert_eq!(render(tpl, json!({ "xs": [] })).trim(), "none");
    assert_eq!(render(tpl, json!({ "xs": ["a", "b"] })), "ab");
    // missing collection behaves like empty
    assert_eq!(render(tpl, json!({})).trim(), "none");
}

// ---------------------------------------------------------------------------
// 6. @continue / @break in loops
// ---------------------------------------------------------------------------

#[test]
fn foreach_break_unconditional() {
    let tpl = "@foreach($xs as $x){{ $x }}@break @endforeach";
    // First item rendered, then break.
    assert_eq!(render(tpl, json!({ "xs": ["a", "b", "c"] })), "a");
}

#[test]
fn foreach_break_conditional() {
    let tpl = "@foreach($xs as $x)@break($x == 3){{ $x }}@endforeach";
    // Renders 1,2 then breaks at 3.
    assert_eq!(render(tpl, json!({ "xs": [1, 2, 3, 4] })), "12");
}

#[test]
fn foreach_continue_conditional() {
    let tpl = "@foreach($xs as $x)@continue($x == 2){{ $x }}@endforeach";
    // Skips 2.
    assert_eq!(render(tpl, json!({ "xs": [1, 2, 3] })), "13");
}

#[test]
fn forelse_with_continue() {
    let tpl = "@forelse($xs as $x)@continue($x == 2){{ $x }}@empty none@endforelse";
    assert_eq!(render(tpl, json!({ "xs": [1, 2, 3] })), "13");
}

// ---------------------------------------------------------------------------
// 7. $loop variable
// ---------------------------------------------------------------------------

#[test]
fn loop_index_iteration() {
    assert_eq!(
        render(
            "@foreach($xs as $x){{ $loop->index }}@endforeach",
            json!({ "xs": [5, 6] })
        ),
        "01"
    );
    assert_eq!(
        render(
            "@foreach($xs as $x){{ $loop->iteration }}@endforeach",
            json!({ "xs": [5, 6] })
        ),
        "12"
    );
    assert_eq!(
        render(
            "@foreach($xs as $x){{ $loop->index0 }}@endforeach",
            json!({ "xs": [5, 6] })
        ),
        "01"
    );
}

#[test]
fn loop_first_last_booleans() {
    // first -> "true" on item 0, "false" on item 1
    assert_eq!(
        render(
            "@foreach($xs as $x){{ $loop->first }}-@endforeach",
            json!({ "xs": [5, 6] })
        ),
        "true-false-"
    );
    assert_eq!(
        render(
            "@foreach($xs as $x){{ $loop->last }}-@endforeach",
            json!({ "xs": [5, 6] })
        ),
        "false-true-"
    );
}

#[test]
fn loop_count_remaining_even_odd() {
    assert_eq!(
        render(
            "@foreach($xs as $x){{ $loop->count }}@endforeach",
            json!({ "xs": [5, 6, 7] })
        ),
        "333"
    );
    // remaining: 2,1,0
    assert_eq!(
        render(
            "@foreach($xs as $x){{ $loop->remaining }}@endforeach",
            json!({ "xs": [5, 6, 7] })
        ),
        "210"
    );
    // even (iteration%2==0): iter 1=false, 2=true, 3=false
    assert_eq!(
        render(
            "@foreach($xs as $x){{ $loop->even }}-@endforeach",
            json!({ "xs": [5, 6, 7] })
        ),
        "false-true-false-"
    );
    // odd
    assert_eq!(
        render(
            "@foreach($xs as $x){{ $loop->odd }}-@endforeach",
            json!({ "xs": [5, 6, 7] })
        ),
        "true-false-true-"
    );
}

#[test]
fn loop_first_usable_in_if() {
    // $loop->first as a condition in @if
    let tpl = "@foreach($xs as $x)@if($loop->first)F@endif{{ $x }}@endforeach";
    assert_eq!(render(tpl, json!({ "xs": ["a", "b"] })), "Fab");
}

#[test]
fn forelse_exposes_loop() {
    let tpl = "@forelse($xs as $x){{ $loop->iteration }}@empty none@endforelse";
    assert_eq!(render(tpl, json!({ "xs": ["a", "b", "c"] })), "123");
}

// ---------------------------------------------------------------------------
// 8. Attribute helpers
// ---------------------------------------------------------------------------

#[test]
fn attribute_helpers() {
    assert_eq!(
        render("@checked($on)", json!({ "on": true })),
        "checked"
    );
    assert_eq!(render("@checked($on)", json!({ "on": false })), "");
    assert_eq!(
        render("@selected($s)", json!({ "s": true })),
        "selected"
    );
    assert_eq!(render("@selected($s)", json!({ "s": false })), "");
    assert_eq!(
        render("@disabled($d)", json!({ "d": true })),
        "disabled"
    );
    assert_eq!(render("@disabled($d)", json!({ "d": false })), "");
    assert_eq!(
        render("@required($r)", json!({ "r": true })),
        "required"
    );
    assert_eq!(render("@required($r)", json!({ "r": false })), "");
    assert_eq!(
        render("@readonly($r)", json!({ "r": true })),
        "readonly"
    );
    assert_eq!(render("@readonly($r)", json!({ "r": false })), "");
}

#[test]
fn attribute_helper_in_context() {
    let tpl = r#"<input type="checkbox" @checked($on)>"#;
    assert_eq!(
        render(tpl, json!({ "on": true })),
        r#"<input type="checkbox" checked>"#
    );
    assert_eq!(
        render(tpl, json!({ "on": false })),
        r#"<input type="checkbox" >"#
    );
}

// ---------------------------------------------------------------------------
// 9. @class / @style
// ---------------------------------------------------------------------------

#[test]
fn class_list() {
    assert_eq!(
        render(
            "@class(['p-4', 'font-bold' => true, 'hidden' => false])",
            json!({})
        ),
        r#"class="p-4 font-bold""#
    );
}

#[test]
fn class_list_with_variable_condition() {
    assert_eq!(
        render(
            "@class(['base', 'active' => $isActive])",
            json!({ "isActive": true })
        ),
        r#"class="base active""#
    );
    assert_eq!(
        render(
            "@class(['base', 'active' => $isActive])",
            json!({ "isActive": false })
        ),
        r#"class="base""#
    );
}

#[test]
fn style_list() {
    assert_eq!(
        render(
            "@style(['color: red', 'font-weight: bold' => true, 'display: none' => false])",
            json!({})
        ),
        r#"style="color: red; font-weight: bold""#
    );
}

// ---------------------------------------------------------------------------
// 10. @once
// ---------------------------------------------------------------------------

#[test]
fn once_renders_body() {
    assert_eq!(render("@once<script>x</script>@endonce", json!({})), "<script>x</script>");
}

// ---------------------------------------------------------------------------
// 11. @php
// ---------------------------------------------------------------------------

#[test]
fn php_renders_nothing() {
    assert_eq!(
        render("before@php $x = 1; echo 'hi'; @endphp after", json!({})),
        "before after"
    );
}

// ---------------------------------------------------------------------------
// Regression: existing @if / @foreach still work alongside new directives.
// ---------------------------------------------------------------------------

#[test]
fn existing_if_foreach_still_work() {
    assert_eq!(
        render("@if($show)yes@else no@endif", json!({ "show": true })),
        "yes"
    );
    assert_eq!(
        render("@foreach($xs as $x){{ $x }}@endforeach", json!({ "xs": ["a", "b", "c"] })),
        "abc"
    );
}
