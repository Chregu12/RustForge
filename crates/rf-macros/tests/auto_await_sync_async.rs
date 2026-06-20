//! End-to-end: #[auto_await] lets you call SYNC and ASYNC framework methods the
//! same way — never writing `.await` — and the macro resolves each correctly.
use rf_macros::auto_await;

// A *synchronous* facade-style API (like rf::Cache / rf::Auth): returns a value
// directly, NOT a future. Method name `get`/`put` are in the auto-await set.
struct Cache;
impl Cache {
    fn put(_k: &str, _v: i64) -> Result<(), String> { Ok(()) }
    fn get(_k: &str) -> Result<Option<i64>, String> { Ok(Some(7)) }
}

// An *asynchronous* model-style API (like the ORM): `find`/`all`/`save` async.
struct User { id: i64 }
impl User {
    async fn find(id: i64) -> Result<User, String> { Ok(User { id }) }
    async fn all() -> Result<Vec<i64>, String> { Ok(vec![1, 2, 3]) }
    async fn save(&self) -> Result<(), String> { Ok(()) }
}

// The developer writes NO `.await` anywhere. `#[auto_await]` decides per call.
#[auto_await]
async fn handler() -> Result<i64, String> {
    Cache::put("k", 9)?;                 // SYNC  -> passthrough (+ ?)
    let cached = Cache::get("k")?;       // SYNC  -> passthrough
    let user = User::find(42)?;          // ASYNC -> awaited
    user.save()?;                        // ASYNC -> awaited
    let ids = User::all()?;              // ASYNC -> awaited
    Ok(cached.unwrap_or(0) + user.id + ids.len() as i64)
}

#[tokio::test(flavor = "multi_thread")]
async fn never_write_await_sync_and_async() {
    let total = handler().await.unwrap();
    assert_eq!(total, 7 + 42 + 3);
}
