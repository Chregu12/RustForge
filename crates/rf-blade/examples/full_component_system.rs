//! Full Blade Component System Example
//!
//! Demonstrates all component features:
//! - Class-based components
//! - Anonymous components
//! - Named slots
//! - Component attributes
//! - Component compiler integration

use rf_blade::components::{
    AttributeBag, BaseComponent, Component, ComponentCompiler, ComponentParser, ComponentProps,
    ComponentRegistry, Slot, SlotBag,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Alert Component (class-based)
#[derive(Debug)]
struct AlertComponent {
    base: BaseComponent,
}

impl AlertComponent {
    fn new() -> Self {
        Self {
            base: BaseComponent::new(
                "alert",
                r#"<div class="alert alert-{{ $type }} {{ $attributes.class }}">
                    {{ $slot }}
                    {% if $dismissible %}
                        <button class="close">&times;</button>
                    {% endif %}
                </div>"#,
            ),
        }
    }
}

impl Component for AlertComponent {
    fn render(
        &self,
        props: &ComponentProps,
        attributes: &AttributeBag,
        slots: &HashMap<String, String>,
    ) -> rf_blade::components::ComponentResult<String> {
        self.base.render(props, attributes, slots)
    }

    fn name(&self) -> &str {
        "alert"
    }
}

/// Card Component with Multiple Slots
#[derive(Debug)]
struct CardComponent {
    base: BaseComponent,
}

impl CardComponent {
    fn new() -> Self {
        Self {
            base: BaseComponent::new(
                "card",
                r#"<div class="card {{ $attributes.class }}">
                    {% if $slots.header %}
                        <div class="card-header">{{ $slots.header }}</div>
                    {% endif %}
                    <div class="card-body">{{ $slot }}</div>
                    {% if $slots.footer %}
                        <div class="card-footer">{{ $slots.footer }}</div>
                    {% endif %}
                </div>"#,
            ),
        }
    }
}

impl Component for CardComponent {
    fn render(
        &self,
        props: &ComponentProps,
        attributes: &AttributeBag,
        slots: &HashMap<String, String>,
    ) -> rf_blade::components::ComponentResult<String> {
        self.base.render(props, attributes, slots)
    }

    fn name(&self) -> &str {
        "card"
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Blade Component System Examples\n");

    // 1. Create component registry
    println!("1️⃣  Setting up component registry...");
    let mut registry = ComponentRegistry::new();

    // Register components
    registry.register("alert", AlertComponent::new())?;
    registry.register("card", CardComponent::new())?;

    let registry = Arc::new(registry);

    // 2. Parse component tags
    println!("\n2️⃣  Parsing component tags...");
    let parser = ComponentParser::new()?;

    let template = r#"
        <x-alert type="danger" dismissible="true">
            This is an error message!
        </x-alert>
    "#;

    let tags = parser.parse_all(template)?;
    println!("   Found {} component(s)", tags.len());

    for tag in &tags {
        println!("   - Component: {}", tag.name);
        println!("     Attributes: {:?}", tag.attributes);
        println!(
            "     Slots: {:?}",
            tag.slots.slot_names()
        );
    }

    // 3. Render components manually
    println!("\n3️⃣  Rendering components manually...");

    let mut props = ComponentProps::new();
    props.set("type".to_string(), serde_json::json!("success"));
    props.set("dismissible".to_string(), serde_json::json!(true));

    let attributes = AttributeBag::new();
    let mut slots = HashMap::new();
    slots.insert("default".to_string(), "Success message!".to_string());

    let rendered = registry.render_component("alert", &props, &attributes, &slots)?;
    println!("   Rendered:\n{}", rendered);

    // 4. Component with named slots
    println!("\n4️⃣  Component with named slots...");

    let mut card_slots = HashMap::new();
    card_slots.insert("header".to_string(), "Card Title".to_string());
    card_slots.insert("default".to_string(), "Card body content".to_string());
    card_slots.insert("footer".to_string(), "Card Footer".to_string());

    let rendered = registry.render_component("card", &ComponentProps::new(), &attributes, &card_slots)?;
    println!("   Rendered:\n{}", rendered);

    // 5. Component compiler integration
    println!("\n5️⃣  Component compiler integration...");

    let compiler = ComponentCompiler::new(Arc::clone(&registry))?;

    let template_with_components = r#"
        <h1>My Page</h1>

        <x-alert type="info">
            Welcome to the application!
        </x-alert>

        <x-card class="mt-4">
            <x-slot name="header">User Profile</x-slot>
            <x-slot name="footer">
                <button>Edit Profile</button>
            </x-slot>

            <p>User information goes here</p>
        </x-card>
    "#;

    let compiled = compiler.compile(template_with_components)?;
    println!("   Compiled template:\n{}", compiled);

    // 6. Nested components
    println!("\n6️⃣  Nested components...");

    let nested_template = r#"
        <x-card>
            <x-slot name="header">Notifications</x-slot>

            <x-alert type="success">
                Profile updated successfully!
            </x-alert>

            <x-alert type="warning">
                Your session will expire in 5 minutes.
            </x-alert>
        </x-card>
    "#;

    let compiled_nested = compiler.compile(nested_template)?;
    println!("   Compiled nested template:\n{}", compiled_nested);

    // 7. Slot system demonstration
    println!("\n7️⃣  Advanced slot system...");

    let mut slot_bag = SlotBag::new();
    slot_bag.set_default("Main content");
    slot_bag.add_slot(Slot::new("header", "Header content"));
    slot_bag.add_slot(Slot::new("footer", "Footer content"));

    println!("   Slot bag contents:");
    println!("   - Default: {:?}", slot_bag.default().map(|s| &s.content));
    println!("   - Header: {:?}", slot_bag.get("header").map(|s| &s.content));
    println!("   - Footer: {:?}", slot_bag.get("footer").map(|s| &s.content));
    println!("   - All slots: {:?}", slot_bag.slot_names());

    println!("\n✅ All examples completed successfully!");

    Ok(())
}
