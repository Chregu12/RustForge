//! Basic search example demonstrating in-memory full-text search

use rf_search::{Document, Query, SearchEngine};

fn main() {
    println!("=== RustForge Full-Text Search Example ===\n");

    // Create a new search engine
    let mut engine = SearchEngine::new();

    // Index some documents
    println!("Indexing documents...");

    engine
        .index(
            Document::new("1")
                .field("title", "Rust Web Framework")
                .field(
                    "content",
                    "Building high-performance web applications with Rust",
                )
                .meta("category", "tutorial")
                .unwrap(),
        )
        .unwrap();

    engine
        .index(
            Document::new("2")
                .field("title", "Python for Beginners")
                .field("content", "Learn Python programming from scratch"),
        )
        .unwrap();

    engine
        .index(
            Document::new("3")
                .field("title", "Rust Systems Programming")
                .field("content", "Deep dive into systems programming with Rust"),
        )
        .unwrap();

    engine
        .index(
            Document::new("4")
                .field("title", "Web Development Guide")
                .field("content", "Complete guide to modern web development"),
        )
        .unwrap();

    println!("Indexed {} documents\n", engine.count());

    // Search for "Rust"
    println!("Searching for 'Rust'...");
    let query = Query::new("Rust");
    let results = engine.search(&query).unwrap();

    println!("Found {} results:", results.len());
    for hit in &results {
        println!(
            "  - [{}] {} (score: {})",
            hit.id, hit.fields["title"], hit.score
        );
    }

    // Search for "web" with pagination
    println!("\nSearching for 'web' (limit 2)...");
    let query = Query::new("web").limit(2);
    let results = engine.search(&query).unwrap();

    println!("Found {} results (showing top 2):", results.len());
    for hit in &results {
        println!(
            "  - [{}] {} (score: {})",
            hit.id, hit.fields["title"], hit.score
        );
    }

    // Search with fuzzy matching
    println!("\nSearching for 'programming' with fuzzy matching...");
    let query = Query::new("programming").fuzzy(0.8).limit(5);
    let results = engine.search(&query).unwrap();

    println!("Found {} results:", results.len());
    for hit in &results {
        println!(
            "  - [{}] {} (score: {})",
            hit.id, hit.fields["title"], hit.score
        );
    }

    println!("\nSearch engine stats:");
    println!("  Documents: {}", engine.count());
    println!("  Terms indexed: {}", engine.term_count());
}
