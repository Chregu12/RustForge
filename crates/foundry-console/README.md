# foundry-console

Console output formatting and styling for Foundry CLI, inspired by Laravel's console output features.

## Features

- **Colors**: Full ANSI color support (foreground and background)
- **Styled Text**: Bold, italic, underline, and dim text
- **Tables**: Beautiful bordered tables with multiple styles
- **Progress Bars**: Visual progress indicators with different styles
- **Lists**: Bullet, numbered, and custom-styled lists
- **Panels**: Bordered content boxes with optional titles
- **Spinners**: Loading animations
- **Status Messages**: Info, success, warning, error, and debug helpers

## Usage

### Colors

```rust
use foundry_console::Colorize;

println!("{}", "Success!".green());
println!("{}", "Error!".red().bold());
println!("{}", "Warning".yellow().on_black());
```

### Tables

```rust
use foundry_console::{Table, TableRow, TableCell, BorderStyle, Colorize};

let mut table = Table::new()
    .with_headers(vec!["Name".to_string(), "Age".to_string()])
    .border_style(BorderStyle::Rounded)
    .title("Users".green().bold());

table.add_row(TableRow::new(vec![
    TableCell::new("Alice"),
    TableCell::new("25"),
]));

println!("{}", table.render());
```

### Progress Bars

```rust
use foundry_console::{ProgressBar, ProgressStyle};

let mut progress = ProgressBar::new(100)
    .with_message("Processing")
    .with_style(ProgressStyle::Bar);

for i in 0..=100 {
    progress.set(i);
    std::thread::sleep(std::time::Duration::from_millis(20));
}

progress.finish();
```

### Lists

```rust
use foundry_console::{List, ListStyle};

let mut list = List::new()
    .with_style(ListStyle::Bullet);

list.add("Item 1");
list.add("Item 2");
list.add("Item 3");

list.print();
```

### Panels

```rust
use foundry_console::{Panel, PanelStyle};

Panel::new("This is important information")
    .with_title("Notice")
    .with_style(PanelStyle::Rounded)
    .print();
```

### Status Messages

```rust
use foundry_console::{info, success, warning, error, header};

info("Processing request...");
success("Request completed successfully!");
warning("Cache is almost full");
error("Connection failed");
header("Configuration");
```

### Spinners

```rust
use foundry_console::{Spinner, SpinnerStyle};

let mut spinner = Spinner::new("Loading")
    .with_style(SpinnerStyle::Dots);

spinner.start();
// Do work...
spinner.stop_with_message("Done!");
```

## Border Styles

Tables and panels support multiple border styles:

- `BorderStyle::Single` - Single-line borders (default)
- `BorderStyle::Double` - Double-line borders
- `BorderStyle::Rounded` - Rounded corners
- `BorderStyle::None` - No borders

## List Styles

Lists support various marker styles:

- `ListStyle::Bullet` - Bullet points (•)
- `ListStyle::Numbered` - Numbers (1., 2., 3.)
- `ListStyle::Dash` - Dashes (-)
- `ListStyle::Arrow` - Arrows (→)
- `ListStyle::Check` - Check marks (✓)

## Progress Styles

Progress bars come in different visual styles:

- `ProgressStyle::Bar` - Filled bar (█░░░)
- `ProgressStyle::Dots` - Dots (●○○○)
- `ProgressStyle::Line` - Line (===--)

## Examples

Run the examples:

```bash
cargo run --example console_features
```

## License

MIT OR Apache-2.0
