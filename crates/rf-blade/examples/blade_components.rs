//! Real-World Blade Component Examples
//!
//! Demonstrates complete component system functionality

use rf_blade::compiler_new::{Compiler, RenderContext};
use rf_blade::components::*;
use rf_blade::parser_new::Parser;
use serde_json::json;
use std::sync::Arc;

fn main() {
    println!("=== Blade Components Phase 2 Examples ===\n");

    // Example 1: Alert Component (class-based)
    example_alert_component();

    // Example 2: Card Component (anonymous-style)
    example_card_component();

    // Example 3: Button Component
    example_button_component();

    // Example 4: Modal Component
    example_modal_component();

    // Example 5: Layout Component with Slots
    example_layout_component();

    // Example 6: Nested Components
    example_nested_components();
}

/// Example 1: Alert Component
fn example_alert_component() {
    println!("Example 1: Alert Component (Class-Based)");
    println!("==========================================");

    let template = r#"<x-alert type="danger" dismissible="true">
        This is a dangerous operation! Are you sure?
    </x-alert>"#;

    // Create alert component
    let alert_template = r#"
<div class="alert alert-{{ $type }} {{ $dismissible == 'true' ? 'alert-dismissible' : '' }}" role="alert">
    {{ $slot }}
    @if($dismissible == 'true')
        <button type="button" class="btn-close" data-bs-dismiss="alert"></button>
    @endif
</div>"#;

    let alert = BaseComponent::new("alert", alert_template);

    // Create registry and register component
    let mut registry = ComponentRegistry::new();
    registry.register("alert", alert).unwrap();

    // Parse and render
    let ast = Parser::parse(template).unwrap();
    let mut context = RenderContext::with_components(json!({}), Arc::new(registry));
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    println!("Template:");
    println!("{}", template);
    println!("\nRendered HTML:");
    println!("{}\n", html);
}

/// Example 2: Card Component
fn example_card_component() {
    println!("Example 2: Card Component with Slots");
    println!("=====================================");

    let template = r#"<x-card>
        @slot('header')
            <h3>User Profile</h3>
        @endslot

        <p>Name: John Doe</p>
        <p>Email: john@example.com</p>

        @slot('footer')
            <button class="btn btn-primary">Edit Profile</button>
        @endslot
    </x-card>"#;

    let card_template = r#"
<div class="card">
    @if(isset($slots.header))
        <div class="card-header">
            {{ $slots.header }}
        </div>
    @endif
    <div class="card-body">
        {{ $slot }}
    </div>
    @if(isset($slots.footer))
        <div class="card-footer">
            {{ $slots.footer }}
        </div>
    @endif
</div>"#;

    let card = BaseComponent::new("card", card_template);

    let mut registry = ComponentRegistry::new();
    registry.register("card", card).unwrap();

    let ast = Parser::parse(template).unwrap();
    let mut context = RenderContext::with_components(json!({}), Arc::new(registry));
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    println!("Template:");
    println!("{}", template);
    println!("\nRendered HTML:");
    println!("{}\n", html);
}

/// Example 3: Button Component
fn example_button_component() {
    println!("Example 3: Button Component");
    println!("============================");

    let template = r#"<x-button type="primary" size="lg" :loading="false">
        Save Changes
    </x-button>"#;

    let button_template = r#"
<button
    class="btn btn-{{ $type }} btn-{{ $size }} {{ $loading == 'true' ? 'loading' : '' }}"
    type="button"
    {{ $loading == 'true' ? 'disabled' : '' }}
>
    @if($loading == 'true')
        <span class="spinner"></span>
    @endif
    {{ $slot }}
</button>"#;

    let button = BaseComponent::new("button", button_template);

    let mut registry = ComponentRegistry::new();
    registry.register("button", button).unwrap();

    let ast = Parser::parse(template).unwrap();
    let mut context = RenderContext::with_components(json!({}), Arc::new(registry));
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    println!("Template:");
    println!("{}", template);
    println!("\nRendered HTML:");
    println!("{}\n", html);
}

/// Example 4: Modal Component
fn example_modal_component() {
    println!("Example 4: Modal Component");
    println!("==========================");

    let template = r#"<x-modal size="md" backdrop="static">
        @slot('title')
            Confirm Deletion
        @endslot

        Are you sure you want to delete this item?

        @slot('footer')
            <button class="btn btn-secondary" data-dismiss="modal">Cancel</button>
            <button class="btn btn-danger">Delete</button>
        @endslot
    </x-modal>"#;

    let modal_template = r#"
<div class="modal fade" tabindex="-1" role="dialog" data-backdrop="{{ $backdrop }}">
    <div class="modal-dialog modal-{{ $size }}">
        <div class="modal-content">
            @if(isset($slots.title))
                <div class="modal-header">
                    <h5 class="modal-title">{{ $slots.title }}</h5>
                    <button type="button" class="close" data-dismiss="modal">
                        <span>&times;</span>
                    </button>
                </div>
            @endif
            <div class="modal-body">
                {{ $slot }}
            </div>
            @if(isset($slots.footer))
                <div class="modal-footer">
                    {{ $slots.footer }}
                </div>
            @endif
        </div>
    </div>
</div>"#;

    let modal = BaseComponent::new("modal", modal_template);

    let mut registry = ComponentRegistry::new();
    registry.register("modal", modal).unwrap();

    let ast = Parser::parse(template).unwrap();
    let mut context = RenderContext::with_components(json!({}), Arc::new(registry));
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    println!("Template:");
    println!("{}", template);
    println!("\nRendered HTML:");
    println!("{}\n", html);
}

/// Example 5: Layout Component
fn example_layout_component() {
    println!("Example 5: Layout Component");
    println!("============================");

    let template = r#"<x-layout>
        @slot('sidebar')
            <nav>
                <ul>
                    <li><a href="/">Home</a></li>
                    <li><a href="/about">About</a></li>
                    <li><a href="/contact">Contact</a></li>
                </ul>
            </nav>
        @endslot

        <h1>Welcome to our site!</h1>
        <p>This is the main content area.</p>
    </x-layout>"#;

    let layout_template = r#"
<div class="layout">
    <div class="sidebar">
        {{ $slots.sidebar }}
    </div>
    <div class="content">
        {{ $slot }}
    </div>
</div>"#;

    let layout = BaseComponent::new("layout", layout_template);

    let mut registry = ComponentRegistry::new();
    registry.register("layout", layout).unwrap();

    let ast = Parser::parse(template).unwrap();
    let mut context = RenderContext::with_components(json!({}), Arc::new(registry));
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    println!("Template:");
    println!("{}", template);
    println!("\nRendered HTML:");
    println!("{}\n", html);
}

/// Example 6: Nested Components
fn example_nested_components() {
    println!("Example 6: Nested Components");
    println!("=============================");

    let template = r#"<x-card>
        @slot('header')
            Dashboard
        @endslot

        <x-alert type="info">
            You have 3 new notifications
        </x-alert>

        <x-button type="primary">
            View All
        </x-button>
    </x-card>"#;

    // Register all components
    let mut registry = ComponentRegistry::new();

    let card = BaseComponent::new(
        "card",
        r#"<div class="card">
            <div class="card-header">{{ $slots.header }}</div>
            <div class="card-body">{{ $slot }}</div>
        </div>"#,
    );

    let alert = BaseComponent::new(
        "alert",
        r#"<div class="alert alert-{{ $type }}">{{ $slot }}</div>"#,
    );

    let button = BaseComponent::new(
        "button",
        r#"<button class="btn btn-{{ $type }}">{{ $slot }}</button>"#,
    );

    registry.register("card", card).unwrap();
    registry.register("alert", alert).unwrap();
    registry.register("button", button).unwrap();

    let ast = Parser::parse(template).unwrap();
    let mut context = RenderContext::with_components(json!({}), Arc::new(registry));
    let compiler = Compiler::new();
    let html = compiler.compile(&ast, &mut context).unwrap();

    println!("Template:");
    println!("{}", template);
    println!("\nRendered HTML:");
    println!("{}\n", html);
}
