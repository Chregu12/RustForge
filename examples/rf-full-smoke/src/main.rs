//! rf-full migration-path smoke test.
//!
//! After the cycle-24 (rc.3) umbrella split, `rf` is core-only. Users who relied
//! on extension surfaces through `rf` migrate with a one-line dependency swap:
//! `rf` → `rf-full`. This example proves that swap resolves BOTH surfaces from
//! the single `rf-full` dependency:
//!   - CORE, via rf-full's `pub use rf::*` (Hash, Route, …)
//!   - EXTENSIONS, via rf-full's extension modules (Blade, Cashier, Nova, …)
//!
//! It is a `cargo check`/run smoke: the extension items are referenced at the
//! type level (the point is that the re-export paths resolve), while a core
//! function is actually called.

// CORE surface — re-exported by rf-full via `pub use rf::*`.
use rf_full::Hash;

// EXTENSION surfaces — the paths that used to live under `rf` and now live
// under `rf-full`. Naming the types is enough to prove the re-exports resolve.
use rf_full::web_ext::blade;
use rf_full::{Cashier, Nova, Passport};

fn main() {
    println!("rf-full migration-path smoke");

    // CORE works through rf-full.
    let hash = Hash::make("secret");
    assert!(Hash::check("secret", &hash));
    println!("  core   ✓  Hash::make/check via rf_full (pub use rf::*)");

    // EXTENSION re-export paths resolve (type-level references).
    let _ = std::any::type_name::<Cashier>();
    let _ = std::any::type_name::<Nova>();
    let _ = std::any::type_name::<Passport>();
    let _ = std::any::type_name::<blade::BladeEngine>();
    println!("  ext    ✓  Cashier / Nova / Passport / blade reachable via rf_full");

    println!("swap `rf` → `rf-full` restores the full pre-rc.3 surface.");
}
