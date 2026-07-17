# foundry-interactive

Interactive prompt system for Foundry CLI, providing Laravel Artisan-inspired interactive features.

## Features

- **Text Input**: Simple and default-value text prompts
- **Choice Selection**: Single-choice menus with arrow navigation
- **Confirmation**: Yes/No prompts
- **Password Input**: Hidden password entry
- **Multi-Select**: Multiple choice selection
- **Autocomplete**: Text input with suggestions

## Usage

### Basic Text Input

```rust
use foundry_interactive::ask;

let name = ask("What is your name?")?;
println!("Hello, {}!", name);
```

### Text Input with Default

```rust
use foundry_interactive::ask_with_default;

let port = ask_with_default("Port", "8080")?;
```

### Choice Selection

```rust
use foundry_interactive::{choice, SelectOption};

let options = vec![
    SelectOption::new("SQLite", "Lightweight database"),
    SelectOption::new("PostgreSQL", "Production database"),
];

let selected = choice("Choose database", &options, 0)?;
```

### Confirmation

```rust
use foundry_interactive::confirm;

if confirm("Overwrite existing file?", false)? {
    // Proceed with overwrite
}
```

### Password Input

```rust
use foundry_interactive::password;

let pwd = password("Enter password")?;
```

### Multi-Select

```rust
use foundry_interactive::{multi_select, SelectOption};

let options = vec![
    SelectOption::simple("Feature A"),
    SelectOption::simple("Feature B"),
    SelectOption::simple("Feature C"),
];

let selected = multi_select("Select features", &options, &[0])?;
```

### Autocomplete

```rust
use foundry_interactive::autocomplete;

let suggestions = vec!["User".to_string(), "Post".to_string(), "Comment".to_string()];
let model = autocomplete("Model name", &suggestions, None)?;
```

## Examples

Run the examples:

```bash
cargo run --example basic_prompts
```

## License

MIT OR Apache-2.0
