//! The framework list is extensible per use via `#[auto_await(also("..."))]`,
//! and now covers more framework async methods out of the box (e.g. broadcast).
use rf_macros::auto_await;

struct Bus;
impl Bus {
    async fn broadcast(_e: &str) -> Result<(), String> { Ok(()) }   // built-in list
    async fn my_custom_op(&self) -> Result<i64, String> { Ok(99) }  // NOT in list
}
struct Cache;
impl Cache { fn get(_k: &str) -> Result<i64, String> { Ok(1) } }    // sync facade

#[auto_await(also("my_custom_op"))]
async fn handler(bus: &Bus) -> Result<i64, String> {
    Bus::broadcast("hello")?;          // framework async (built-in) -> awaited
    let c = Cache::get("k")?;          // sync facade -> passthrough
    let v = bus.my_custom_op()?;       // custom async via `also(...)` -> awaited
    Ok(c + v)
}

#[tokio::test(flavor = "multi_thread")]
async fn extend_with_also() {
    assert_eq!(handler(&Bus).await.unwrap(), 1 + 99);
}
