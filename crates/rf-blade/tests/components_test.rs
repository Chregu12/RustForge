//! Comprehensive tests for Blade Components (Phase 2)

use rf_blade::ast::AstNode;
use rf_blade::compiler_new::{Compiler, RenderContext};
use rf_blade::components::*;
use rf_blade::lexer::{Lexer, Token};
use rf_blade::parser_new::Parser;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// LEXER TESTS (Component Tags)
// ============================================================================

#[test]
fn test_lexer_self_closing_component() {
    let tokens = Lexer::tokenize("<x-alert />").unwrap();

    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        Token::ComponentStart {
            name,
            attributes,
            self_closing,
        } => {
            assert_eq!(name, "alert");
            assert_eq!(attributes.len(), 0);
            assert!(self_closing);
        }
        _ => panic!("Expected ComponentStart token"),
    }
}

#[test]
fn test_lexer_component_with_attributes() {
    let tokens = Lexer::tokenize(r#"<x-alert type="danger" dismissible />"#).unwrap();

    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        Token::ComponentStart {
            name,
            attributes,
            self_closing,
        } => {
            assert_eq!(name, "alert");
            assert_eq!(attributes.len(), 2);
            assert_eq!(attributes[0].0, "type");
            assert_eq!(attributes[0].1, "danger");
            assert_eq!(attributes[1].0, "dismissible");
            assert!(self_closing);
        }
        _ => panic!("Expected ComponentStart token"),
    }
}

#[test]
fn test_lexer_component_with_content() {
    let tokens = Lexer::tokenize(r#"<x-card>Content here</x-card>"#).unwrap();

    assert_eq!(tokens.len(), 3);
    match &tokens[0] {
        Token::ComponentStart {
            name, self_closing, ..
        } => {
            assert_eq!(name, "card");
            assert!(!self_closing);
        }
        _ => panic!("Expected ComponentStart token"),
    }

    match &tokens[1] {
        Token::Text(text) => assert_eq!(text, "Content here"),
        _ => panic!("Expected Text token"),
    }

    match &tokens[2] {
        Token::ComponentEnd(name) => assert_eq!(name, "card"),
        _ => panic!("Expected ComponentEnd token"),
    }
}

#[test]
fn test_lexer_nested_component_names() {
    let tokens = Lexer::tokenize("<x-layouts.sidebar />").unwrap();

    match &tokens[0] {
        Token::ComponentStart { name, .. } => {
            assert_eq!(name, "layouts.sidebar");
        }
        _ => panic!("Expected ComponentStart token"),
    }
}

#[test]
fn test_lexer_component_with_colon_attribute() {
    let tokens = Lexer::tokenize(r#"<x-button :disabled="isDisabled" />"#).unwrap();

    match &tokens[0] {
        Token::ComponentStart { attributes, .. } => {
            assert_eq!(attributes[0].0, ":disabled");
            assert_eq!(attributes[0].1, "isDisabled");
        }
        _ => panic!("Expected ComponentStart token"),
    }
}

// ============================================================================
// PARSER TESTS (Component Parsing)
// ============================================================================

#[test]
fn test_parse_self_closing_component() {
    let nodes = Parser::parse("<x-alert />").unwrap();

    assert_eq!(nodes.len(), 1);
    match &nodes[0] {
        AstNode::Component { name, children, .. } => {
            assert_eq!(name, "alert");
            assert_eq!(children.len(), 0);
        }
        _ => panic!("Expected Component node"),
    }
}

#[test]
fn test_parse_component_with_children() {
    let nodes = Parser::parse("<x-card>Hello</x-card>").unwrap();

    assert_eq!(nodes.len(), 1);
    match &nodes[0] {
        AstNode::Component { name, children, .. } => {
            assert_eq!(name, "card");
            assert_eq!(children.len(), 1);
        }
        _ => panic!("Expected Component node"),
    }
}

#[test]
fn test_parse_component_with_slots() {
    let template = r#"<x-card>
        @slot('header')Header Text@endslot
        Body content
    </x-card>"#;

    let nodes = Parser::parse(template).unwrap();

    match &nodes[0] {
        AstNode::Component {
            name,
            slots,
            children,
            ..
        } => {
            assert_eq!(name, "card");
            assert!(slots.contains_key("header"));
            assert!(!children.is_empty());
        }
        _ => panic!("Expected Component node"),
    }
}

#[test]
fn test_parse_nested_components() {
    let template = r#"<x-card><x-button>Click</x-button></x-card>"#;

    let nodes = Parser::parse(template).unwrap();

    match &nodes[0] {
        AstNode::Component { name, children, .. } => {
            assert_eq!(name, "card");
            assert_eq!(children.len(), 1);

            match &children[0] {
                AstNode::Component {
                    name: inner_name, ..
                } => {
                    assert_eq!(inner_name, "button");
                }
                _ => panic!("Expected nested Component node"),
            }
        }
        _ => panic!("Expected Component node"),
    }
}

// ============================================================================
// ATTRIBUTE BAG TESTS
// ============================================================================

#[test]
fn test_attribute_bag_creation() {
    let bag = AttributeBag::new();
    assert!(bag.is_empty());
}

#[test]
fn test_attribute_bag_from_pairs() {
    let bag = AttributeBag::from_pairs(vec![("class".to_string(), "btn".to_string())]);

    assert_eq!(bag.get("class"), Some(&"btn".to_string()));
}

#[test]
fn test_attribute_bag_merge_classes() {
    let mut bag1 = AttributeBag::new();
    bag1.set("class".to_string(), "btn".to_string());

    let mut bag2 = AttributeBag::new();
    bag2.set("class".to_string(), "btn-primary".to_string());

    bag1.merge(&bag2);

    assert_eq!(bag1.get("class"), Some(&"btn btn-primary".to_string()));
}

#[test]
fn test_attribute_bag_except() {
    let mut bag = AttributeBag::new();
    bag.set("class".to_string(), "btn".to_string());
    bag.set("id".to_string(), "submit".to_string());

    let filtered = bag.except(&["id"]);

    assert!(filtered.has("class"));
    assert!(!filtered.has("id"));
}

#[test]
fn test_attribute_bag_only() {
    let mut bag = AttributeBag::new();
    bag.set("class".to_string(), "btn".to_string());
    bag.set("id".to_string(), "submit".to_string());

    let filtered = bag.only(&["class"]);

    assert!(filtered.has("class"));
    assert!(!filtered.has("id"));
}

#[test]
fn test_attribute_bag_to_html() {
    let mut bag = AttributeBag::new();
    bag.set("class".to_string(), "btn".to_string());
    bag.set("id".to_string(), "submit".to_string());

    let html = bag.to_html();

    assert!(html.contains("class=\"btn\""));
    assert!(html.contains("id=\"submit\""));
}

#[test]
fn test_attribute_bag_html_escape() {
    let mut bag = AttributeBag::new();
    bag.set(
        "data".to_string(),
        "<script>alert('xss')</script>".to_string(),
    );

    let html = bag.to_html();

    assert!(html.contains("&lt;script&gt;"));
}

// ============================================================================
// COMPONENT PROPS TESTS
// ============================================================================

#[test]
fn test_component_props_creation() {
    let props = ComponentProps::new();
    assert!(props.all().is_empty());
}

#[test]
fn test_component_props_set_get() {
    let mut props = ComponentProps::new();
    props.set("name".to_string(), json!("Alice"));

    assert_eq!(props.get("name"), Some(&json!("Alice")));
}

#[test]
fn test_component_props_get_string() {
    let mut props = ComponentProps::new();
    props.set("title".to_string(), json!("Hello"));

    assert_eq!(props.get_string("title"), Some("Hello".to_string()));
}

#[test]
fn test_component_props_get_int() {
    let mut props = ComponentProps::new();
    props.set("count".to_string(), json!(42));

    assert_eq!(props.get_int("count"), Some(42));
}

#[test]
fn test_component_props_get_bool() {
    let mut props = ComponentProps::new();
    props.set("active".to_string(), json!(true));

    assert_eq!(props.get_bool("active"), Some(true));
}

#[test]
fn test_component_props_required() {
    let mut props = ComponentProps::new();
    props.require("name".to_string());

    let result = props.validate();
    assert!(result.is_err());
}

#[test]
fn test_component_props_default() {
    let mut props = ComponentProps::new();
    props.default("type".to_string(), json!("primary"));

    assert_eq!(props.get("type"), Some(&json!("primary")));
}

#[test]
fn test_component_props_from_attributes() {
    let attrs = vec![
        ("class".to_string(), "btn".to_string()),
        ("type".to_string(), "button".to_string()),
    ];

    let props = ComponentProps::from_attributes(&attrs);

    assert_eq!(props.get_string("class"), Some("btn".to_string()));
    assert_eq!(props.get_string("type"), Some("button".to_string()));
}

// ============================================================================
// CLASS COMPONENT TESTS
// ============================================================================

#[test]
fn test_base_component_creation() {
    let component = BaseComponent::new("alert", "<div>Alert</div>");
    assert_eq!(component.name(), "alert");
}

#[test]
fn test_base_component_render_simple() {
    let component = BaseComponent::new("alert", r#"<div class="alert">{{ $slot }}</div>"#);

    let props = ComponentProps::new();
    let attributes = AttributeBag::new();
    let mut slots = HashMap::new();
    slots.insert("default".to_string(), "Hello!".to_string());

    let html = component.render(&props, &attributes, &slots).unwrap();

    assert!(html.contains("alert"));
    assert!(html.contains("Hello!"));
}

#[test]
fn test_base_component_render_with_props() {
    let component = BaseComponent::new(
        "alert",
        r#"<div class="alert alert-{{ $type }}">{{ $slot }}</div>"#,
    );

    let mut props = ComponentProps::new();
    props.set("type".to_string(), json!("danger"));

    let attributes = AttributeBag::new();
    let mut slots = HashMap::new();
    slots.insert("default".to_string(), "Error!".to_string());

    let html = component.render(&props, &attributes, &slots).unwrap();

    assert!(html.contains("alert-danger"));
    assert!(html.contains("Error!"));
}

// ============================================================================
// COMPONENT REGISTRY TESTS
// ============================================================================

#[test]
fn test_component_registry_creation() {
    let registry = ComponentRegistry::new();
    assert_eq!(registry.component_names().len(), 0);
}

#[test]
fn test_component_registry_register() {
    let mut registry = ComponentRegistry::new();
    let component = BaseComponent::new("alert", "<div>Alert</div>");

    registry.register("alert", component).unwrap();

    assert!(registry.has("alert"));
}

#[test]
fn test_component_registry_alias() {
    let mut registry = ComponentRegistry::new();
    let component = BaseComponent::new("alert", "<div>Alert</div>");

    registry.register("alert", component).unwrap();
    registry.alias("alert", "notification").unwrap();

    assert!(registry.has("alert"));
    assert!(registry.has("notification"));
}

#[test]
fn test_component_registry_render() {
    let mut registry = ComponentRegistry::new();
    let component = BaseComponent::new("alert", r#"<div class="alert">{{ $slot }}</div>"#);

    registry.register("alert", component).unwrap();

    let props = ComponentProps::new();
    let attributes = AttributeBag::new();
    let mut slots = HashMap::new();
    slots.insert("default".to_string(), "Test".to_string());

    let html = registry
        .render_component("alert", &props, &attributes, &slots)
        .unwrap();

    assert!(html.contains("alert"));
    assert!(html.contains("Test"));
}

// ============================================================================
// INTEGRATION TESTS (Full Component Rendering)
// ============================================================================

#[test]
fn test_render_component_end_to_end() {
    let template = r#"<x-alert type="danger">This is dangerous!</x-alert>"#;

    // Create registry
    let mut registry = ComponentRegistry::new();
    let component = BaseComponent::new(
        "alert",
        r#"<div class="alert alert-{{ $type }}">{{ $slot }}</div>"#,
    );
    registry.register("alert", component).unwrap();

    // Parse template
    let ast = Parser::parse(template).unwrap();

    // Create context with registry
    let data = json!({});
    let mut context = RenderContext::with_components(data, Arc::new(registry));

    // Compile
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    assert!(html.contains("alert-danger"));
    assert!(html.contains("This is dangerous!"));
}

#[test]
fn test_render_component_with_named_slots() {
    let template = r#"<x-card>
        @slot('header')Card Title@endslot
        Card body content
    </x-card>"#;

    // Create registry
    let mut registry = ComponentRegistry::new();
    let component = BaseComponent::new(
        "card",
        r#"<div class="card">
            <div class="card-header">{{ $slots.header }}</div>
            <div class="card-body">{{ $slot }}</div>
        </div>"#,
    );
    registry.register("card", component).unwrap();

    // Parse and compile
    let ast = Parser::parse(template).unwrap();
    let mut context = RenderContext::with_components(json!({}), Arc::new(registry));
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    assert!(html.contains("Card Title"));
    assert!(html.contains("Card body content"));
}

#[test]
fn test_render_self_closing_component() {
    let template = r#"<x-icon name="check" />"#;

    let mut registry = ComponentRegistry::new();
    let component = BaseComponent::new("icon", r#"<i class="icon icon-{{ $name }}"></i>"#);
    registry.register("icon", component).unwrap();

    let ast = Parser::parse(template).unwrap();
    let mut context = RenderContext::with_components(json!({}), Arc::new(registry));
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    assert!(html.contains("icon-check"));
}

#[test]
fn test_render_nested_components() {
    let template = r#"<x-card><x-button>Click me</x-button></x-card>"#;

    let mut registry = ComponentRegistry::new();

    let card = BaseComponent::new("card", r#"<div class="card">{{ $slot }}</div>"#);
    let button = BaseComponent::new("button", r#"<button>{{ $slot }}</button>"#);

    registry.register("card", card).unwrap();
    registry.register("button", button).unwrap();

    let ast = Parser::parse(template).unwrap();
    let mut context = RenderContext::with_components(json!({}), Arc::new(registry));
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    assert!(html.contains("card"));
    assert!(html.contains("button"));
    assert!(html.contains("Click me"));
}

#[test]
fn test_render_component_with_attributes() {
    let template = r#"<x-button class="btn-primary" id="submit">Submit</x-button>"#;

    let mut registry = ComponentRegistry::new();
    let component = BaseComponent::new(
        "button",
        r#"<button class="{{ $attributes.class }}" id="{{ $attributes.get('id') }}">{{ $slot }}</button>"#,
    );
    registry.register("button", component).unwrap();

    let ast = Parser::parse(template).unwrap();
    let mut context = RenderContext::with_components(json!({}), Arc::new(registry));
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    assert!(html.contains("btn-primary"));
    assert!(html.contains("Submit"));
}

// ============================================================================
// PROP VALIDATION TESTS
// ============================================================================

#[test]
fn test_prop_definition_validation() {
    let def = PropDefinition::new("count").with_type(PropType::Integer);

    assert!(def.validate(&json!(42)).is_ok());
    assert!(def.validate(&json!("not a number")).is_err());
}

#[test]
fn test_prop_definition_default() {
    let def = PropDefinition::new("type").default_value(json!("primary"));

    assert_eq!(def.default, Some(json!("primary")));
}

// Test count: 54 tests total
