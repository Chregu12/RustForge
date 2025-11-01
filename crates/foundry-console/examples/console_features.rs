use foundry_console::{
    Table, TableRow, TableCell, BorderStyle, Colorize,
    ProgressBar, ProgressStyle,
    List, ListStyle,
    Panel, PanelStyle,
    info, success, warning, error, header, line,
    bold, dim,
};
use std::thread;
use std::time::Duration;

fn main() {
    println!("\n=== Foundry Console Features Demo ===\n");

    // Section headers
    header("Colors and Styles");
    line(&"Red text".red());
    line(&"Green text".green());
    line(&"Yellow text".yellow());
    line(&"Blue text".blue());
    line(&"Bold text".bold());
    line(&"Dim text".dim());
    println!();

    // Status messages
    header("Status Messages");
    info("This is an info message");
    success("This is a success message");
    warning("This is a warning message");
    error("This is an error message");
    println!();

    // Tables
    header("Tables");
    let mut table = Table::new()
        .with_headers(vec![
            "Command".to_string(),
            "Category".to_string(),
            "Description".to_string(),
        ])
        .border_style(BorderStyle::Rounded)
        .title("Available Commands".green().bold());

    table.add_row(TableRow::new(vec![
        TableCell::new("migrate".bold()),
        TableCell::new("Database".cyan()),
        TableCell::new("Run database migrations"),
    ]));

    table.add_row(TableRow::new(vec![
        TableCell::new("make:model".bold()),
        TableCell::new("Generator".green()),
        TableCell::new("Generate a new model"),
    ]));

    table.add_row(TableRow::new(vec![
        TableCell::new("serve".bold()),
        TableCell::new("Server".magenta()),
        TableCell::new("Start the HTTP server"),
    ]));

    println!("{}", table.render());
    println!();

    // Lists
    header("Lists");
    let mut list = List::new().with_style(ListStyle::Bullet);
    list.add("First item");
    list.add("Second item");
    list.add("Third item");
    list.print();
    println!();

    let mut numbered_list = List::new().with_style(ListStyle::Numbered);
    numbered_list.add("Step one");
    numbered_list.add("Step two");
    numbered_list.add("Step three");
    numbered_list.print();
    println!();

    // Panels
    header("Panels");
    Panel::new("This is a simple panel with default styling")
        .print();
    println!();

    Panel::new("This is a panel with a title")
        .with_title("Information".bold())
        .with_style(PanelStyle::Rounded)
        .print();
    println!();

    // Progress Bar
    header("Progress Bars");
    println!("Bar style:");
    let mut progress = ProgressBar::new(100)
        .with_message("Processing items")
        .with_style(ProgressStyle::Bar);

    for i in 0..=100 {
        progress.set(i);
        thread::sleep(Duration::from_millis(20));
    }
    progress.finish_with_message("Complete!");

    println!("\nDots style:");
    let mut progress = ProgressBar::new(50)
        .with_message("Loading data")
        .with_style(ProgressStyle::Dots);

    for i in 0..=50 {
        progress.set(i);
        thread::sleep(Duration::from_millis(30));
    }
    progress.finish();

    println!("\n=== Demo Complete! ===\n");
}
