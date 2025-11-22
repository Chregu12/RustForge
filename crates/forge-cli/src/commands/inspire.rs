//! Inspiration command
//!
//! Display an inspiring quote

use colored::*;

/// Display an inspiring quote
pub fn run() {
    let quotes = vec![
        ("Be yourself; everyone else is already taken.", "Oscar Wilde"),
        ("So many books, so little time.", "Frank Zappa"),
        ("Two things are infinite: the universe and human stupidity.", "Albert Einstein"),
        ("A room without books is like a body without a soul.", "Marcus Tullius Cicero"),
        ("Be the change that you wish to see in the world.", "Mahatma Gandhi"),
        ("If you tell the truth, you don't have to remember anything.", "Mark Twain"),
        ("Without requirements or design, programming is the art of adding bugs to an empty text file.", "Louis Srygley"),
        ("Before software can be reusable it first has to be usable.", "Ralph Johnson"),
        ("Code is like humor. When you have to explain it, it's bad.", "Cory House"),
        ("First, solve the problem. Then, write the code.", "John Johnson"),
        ("Simplicity is the soul of efficiency.", "Austin Freeman"),
        ("Make it work, make it right, make it fast.", "Kent Beck"),
        ("Clean code always looks like it was written by someone who cares.", "Robert C. Martin"),
        ("Any fool can write code that a computer can understand. Good programmers write code that humans can understand.", "Martin Fowler"),
        ("Truth can only be found in one place: the code.", "Robert C. Martin"),
        ("The best error message is the one that never shows up.", "Thomas Fuchs"),
        ("Rust: A language empowering everyone to build reliable and efficient software.", "The Rust Team"),
        ("Perfection is achieved not when there is nothing more to add, but when there is nothing left to take away.", "Antoine de Saint-Exupéry"),
        ("Talk is cheap. Show me the code.", "Linus Torvalds"),
        ("Walking on water and developing software from a specification are easy if both are frozen.", "Edward V. Berard"),
    ];

    let idx = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        % quotes.len();

    let (quote, author) = quotes[idx];

    println!();
    println!("  {}", quote.italic());
    println!();
    println!(
        "      {} {}",
        "—".bright_black(),
        author.bright_black().italic()
    );
    println!();
}
