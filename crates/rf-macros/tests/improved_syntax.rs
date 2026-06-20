//! The improved, string-free syntax: identifiers via #[await_calls(..)], and a
//! module entry where handlers look synchronous (auto-made async + auto-await).
use rf_macros::{auto_await, await_calls};

struct Cache; impl Cache { fn put(_k: &str, _v: i64) -> Result<(), String> { Ok(()) } }
struct User; impl User { async fn find(_id: i64) -> Result<i64, String> { Ok(7) } }
async fn fetch_report() -> i64 { 100 }
async fn charge(_amount: i64) -> i64 { 50 }

// 1) String-free extension with bare identifiers:
#[await_calls(fetch_report, charge)]
fn checkout() -> Result<i64, String> {        // sync-looking; macro makes it async
    let report = fetch_report();              // custom async -> awaited
    let c = charge(10);                        // custom async -> awaited
    let u = User::find(1)?;                    // framework async -> awaited
    Cache::put("done", 1)?;                    // framework sync   -> passed through
    Ok(report + c + u)
}

// 2) Module entry: handlers written as plain `fn`, all auto-async + auto-await:
#[auto_await]
mod app {
    use super::*;
    pub fn index() -> Result<i64, String> {    // no `async`, no `.await`
        let u = User::find(1)?;
        Cache::put("hit", 1)?;
        Ok(u)
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn improved() {
    assert_eq!(checkout().await.unwrap(), 100 + 50 + 7);
    assert_eq!(app::index().await.unwrap(), 7);
}
