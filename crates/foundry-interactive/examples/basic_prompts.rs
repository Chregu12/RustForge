use foundry_interactive::{ask, ask_with_default, choice, confirm, password, multi_select, SelectOption};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Foundry Interactive Prompts Examples ===\n");

    // Simple text input
    let name = ask("What is your name?")?;
    println!("Hello, {}!\n", name);

    // Text input with default
    let port = ask_with_default("Port", "8080")?;
    println!("Using port: {}\n", port);

    // Single choice selection
    let db_options = vec![
        SelectOption::new("SQLite", "Lightweight embedded database"),
        SelectOption::new("PostgreSQL", "Production-ready database"),
        SelectOption::new("MySQL", "Popular relational database"),
    ];
    let database = choice("Choose your database", &db_options, 0)?;
    println!("Selected database: {}\n", database);

    // Confirmation
    let proceed = confirm("Do you want to continue?", true)?;
    if !proceed {
        println!("Operation cancelled.");
        return Ok(());
    }

    // Multi-select
    let feature_options = vec![
        SelectOption::simple("Authentication"),
        SelectOption::simple("API Integration"),
        SelectOption::simple("Caching"),
        SelectOption::simple("Queue System"),
    ];
    let features = multi_select("Select features to enable", &feature_options, &[0, 2])?;
    println!("Enabled features: {:?}\n", features);

    // Password input
    let pwd = password("Enter a password")?;
    println!("Password length: {} characters\n", pwd.len());

    println!("=== All done! ===");

    Ok(())
}
