//! `#[auto_await]` on a *non-async* fn auto-converts it to async (it then
//! returns impl Future), per ChatGPT's note — so you don't even write `async`.
use rf_macros::auto_await;

struct Cache;
impl Cache { fn get(_k: &str) -> Result<i64, String> { Ok(5) } }   // sync
async fn afind() -> i64 { 10 }                                      // async

#[auto_await(also("afind"))]
fn handler() -> Result<i64, String> {            // <-- note: NO `async` keyword
    let c = Cache::get("k")?;
    let a = afind();
    Ok(c + a)
}

#[tokio::test(flavor = "multi_thread")]
async fn fn_becomes_async() {
    // handler() now returns a Future because the macro made it async.
    let total = handler().await.unwrap();
    assert_eq!(total, 15);
}
