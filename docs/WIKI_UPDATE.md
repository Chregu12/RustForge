# RustForge Wiki Features Update - Sync API Migration

> **Phase 21 Update**: RustForge hat die API auf synchrone Methoden umgestellt für maximale Laravel-Parität und Developer Experience.

## Überblick der Änderungen

RustForge bietet jetzt eine **vollständig synchrone Public API** während intern weiterhin async/await verwendet wird:

- ✅ **Keine `.await` mehr nötig** - Einfacherer, Laravel-ähnlicher Code
- ✅ **Konsolidierte Imports** - `use rf::prelude::*;` für alles
- ✅ **Laravel-Parität** - Exakt die gleiche API wie Laravel PHP
- ✅ **Intern async** - Performance bleibt unverändert

---

## 📦 Neue Import-Struktur

### Vorher (Phase 20)
```rust
use rf_auth::Auth;
use rf_cache::Cache;
use rf_db::DB;
use rf_storage::Storage;
// ... viele einzelne Imports
```

### Jetzt (Phase 21)
```rust
// Einfachste Option - Prelude für alles
use rf::prelude::*;

// Oder spezifische Imports
use rf::{Auth, DB, Cache, Storage, Hash, Collection};

// Oder nach Modulen organisiert
use rf::web::*;        // HTTP, Views, API
use rf::data::*;       // DB, Cache, Validation
use rf::background::*; // Jobs, Events, Broadcast
use rf::services::*;   // Storage, Mail, Auth
```

---

## 1. Authentication & Authorization

### Sync API ohne .await

#### Benutzer Authentifizierung

```rust
use rf::prelude::*;

// Login
fn login(email: String, password: String) -> Result<String> {
    // Benutzer finden - kein .await nötig!
    let user = DB::table("users")
        .where("email", email)
        .first()?;

    // Passwort verifizieren
    if !Hash::check(&password, &user.password) {
        return Err("Invalid credentials".into());
    }

    // Token generieren
    let token = Auth::login(&user)?;
    Ok(token)
}

// Aktuellen Benutzer abrufen
fn current_user() -> User {
    Auth::user()
}

// Logout
fn logout() {
    Auth::logout();
}

// Authentifizierung prüfen
fn check_auth() -> bool {
    Auth::check()
}
```

#### Guards & Multi-Auth

```rust
use rf::prelude::*;

// Standard Guard (session)
let user = Auth::user();

// API Guard (token)
let api_user = Auth::guard("api").user();

// Attempt Login
if Auth::attempt(credentials) {
    println!("Login successful");
}

// Login mit "Remember Me"
Auth::attempt(credentials).remember(true);
```

#### Authorization (Gates & Policies)

```rust
use rf::prelude::*;
use rf::services::auth::{Gate, Policy};

// Gates definieren
Gate::define("update-post", |user, post: &Post| {
    user.id == post.user_id
});

// Gate prüfen - kein .await!
if Gate::allows("update-post", &post) {
    post.update(data)?;
}

// Policy verwenden
if Auth::user().can("update", &post) {
    // Update erlaubt
}

// Policy-basierte Middleware
Route::put("/posts/:id")
    .middleware(can("update", "Post"));
```

#### Password Reset

```rust
use rf::prelude::*;

// Reset Token senden
fn send_reset_link(email: String) -> Result<()> {
    Auth::send_reset_link(&email)
}

// Passwort zurücksetzen
fn reset_password(token: String, email: String, password: String) -> Result<()> {
    Auth::reset_password(token, email, password)
}
```

#### Email Verification

```rust
use rf::prelude::*;

// Verification Email senden
fn send_verification(user: &User) -> Result<()> {
    Auth::send_verification(user)
}

// Email verifizieren
fn verify_email(token: String) -> Result<()> {
    Auth::verify_email(token)
}

// Verifizierung prüfen
fn must_verify() -> bool {
    Auth::user().email_verified_at.is_none()
}
```

---

## 2. Database & ORM

### Query Builder - Synchrone API

```rust
use rf::prelude::*;

// Alle Datensätze
let users = DB::table("users").get();

// Mit Where-Bedingung
let active_users = DB::table("users")
    .where("active", true)
    .get();

// Einzelner Datensatz
let user = DB::table("users")
    .where("email", "john@example.com")
    .first()?;

// Insert
DB::table("users").insert(json!({
    "name": "John Doe",
    "email": "john@example.com"
}))?;

// Update
DB::table("users")
    .where("id", 1)
    .update(json!({"name": "Jane Doe"}))?;

// Delete
DB::table("users")
    .where("id", 1)
    .delete()?;

// Aggregationen
let count = DB::table("users").count();
let avg_age = DB::table("users").avg("age");
let max_score = DB::table("users").max("score");
```

### Eloquent ORM - Model API

```rust
use rf::prelude::*;

// Model definieren
#[derive(Model)]
struct User {
    id: i64,
    name: String,
    email: String,
    created_at: DateTime,
}

// Alle abrufen
let users = User::all();

// Mit Bedingungen
let active = User::where("active", true).get();

// Einzelnes Model
let user = User::find(1)?;
let user = User::where("email", email).first()?;

// Erstellen
let user = User::create(json!({
    "name": "John",
    "email": "john@example.com"
}))?;

// Aktualisieren
user.name = "Jane".to_string();
user.save()?;

// Löschen
user.delete()?;

// Soft Deletes
user.soft_delete()?;
let with_deleted = User::with_trashed().get();
user.restore()?;
```

### Relationships - Sync API

```rust
use rf::prelude::*;

// Relationships laden
let user = User::find(1)?;
let posts = user.posts();
let latest_post = user.posts().latest().first()?;

// Eager Loading
let users = User::with(vec!["posts", "comments"]).get();

// Lazy Eager Loading
user.load("posts");

// Has Relationship
let users_with_posts = User::has("posts").get();
let users_with_5_posts = User::has("posts", ">", 5).get();

// Where Has
let users = User::where_has("posts", |query| {
    query.where("published", true)
}).get();
```

---

## 3. Caching

### Synchrone Cache-API

```rust
use rf::prelude::*;

// Wert abrufen
let value: Option<String> = Cache::get("key");

// Mit Default
let value = Cache::get_or("key", "default");

// Wert setzen
Cache::put("key", "value", 3600); // 3600 Sekunden TTL

// Forever speichern
Cache::forever("key", "value");

// Remember (get or set)
let users = Cache::remember("users", 3600, || {
    DB::table("users").get()
});

// Increment/Decrement
Cache::increment("counter", 1);
Cache::decrement("counter", 1);

// Löschen
Cache::forget("key");

// Flush (alles löschen)
Cache::flush();

// Prüfen ob existiert
if Cache::has("key") {
    println!("Key exists");
}
```

### Cache Tags

```rust
use rf::prelude::*;

// Mit Tags speichern
Cache::tags(vec!["users", "premium"])
    .put("user:1", user, 3600);

// Tags abrufen
let user = Cache::tags(vec!["users"]).get("user:1");

// Tags löschen (flush)
Cache::tags(vec!["users"]).flush();
```

### Cache Drivers

```rust
use rf::prelude::*;

// Standard Driver (aus config)
Cache::put("key", "value", 3600);

// Spezifischer Driver
Cache::driver("redis").put("key", "value", 3600);
Cache::driver("memcached").get("key");
Cache::driver("file").flush();
```

---

## 4. Storage

### Synchrone File Storage API

```rust
use rf::prelude::*;

// Datei speichern
Storage::put("avatars/user.jpg", file_content)?;

// Datei abrufen
let content = Storage::get("avatars/user.jpg")?;

// Datei existiert?
if Storage::exists("avatars/user.jpg") {
    println!("File exists");
}

// Datei löschen
Storage::delete("avatars/user.jpg")?;

// Datei verschieben
Storage::move_file("old/path.jpg", "new/path.jpg")?;

// Datei kopieren
Storage::copy("source.jpg", "destination.jpg")?;

// URL generieren
let url = Storage::url("avatars/user.jpg");

// Temporäre signierte URL
let url = Storage::temporary_url("private/file.pdf", 3600)?;

// Dateigröße
let size = Storage::size("file.jpg")?;

// Letztes Änderungsdatum
let modified = Storage::last_modified("file.jpg")?;
```

### Storage Disks

```rust
use rf::prelude::*;

// Standard Disk (aus config)
Storage::put("file.txt", content)?;

// Spezifischer Disk
Storage::disk("s3").put("file.txt", content)?;
Storage::disk("local").get("file.txt")?;
Storage::disk("public").url("image.jpg");

// Alle Dateien auflisten
let files = Storage::disk("s3").files("avatars")?;
let all_files = Storage::disk("s3").all_files("avatars")?;

// Verzeichnisse
let dirs = Storage::directories("/")?;
Storage::make_directory("new-folder")?;
Storage::delete_directory("old-folder")?;
```

### File Upload

```rust
use rf::prelude::*;
use rf::web::Request;

fn upload_avatar(request: Request) -> Result<String> {
    // Datei aus Request holen
    let file = request.file("avatar")?;

    // Validierung
    if file.size() > 2_000_000 {
        return Err("File too large".into());
    }

    // Speichern
    let path = file.store("avatars")?;

    // Mit Custom Name
    let path = file.store_as("avatars", "custom-name.jpg")?;

    Ok(path)
}
```

---

## 5. Mail

### Synchrone Mail API

```rust
use rf::prelude::*;

// Einfache Mail senden
Mail::to("user@example.com")
    .subject("Welcome!")
    .text("Welcome to our platform")
    .send()?;

// HTML Mail
Mail::to("user@example.com")
    .subject("Welcome!")
    .html("<h1>Welcome!</h1>")
    .send()?;

// Mit Template
Mail::to("user@example.com")
    .subject("Welcome!")
    .view("emails.welcome", json!({"name": "John"}))
    .send()?;

// Mehrere Empfänger
Mail::to(vec!["user1@example.com", "user2@example.com"])
    .cc("manager@example.com")
    .bcc("admin@example.com")
    .subject("Team Update")
    .send()?;

// Mit Anhängen
Mail::to("user@example.com")
    .subject("Invoice")
    .attach("invoices/2024-01.pdf")?
    .send()?;
```

### Mailable Classes

```rust
use rf::prelude::*;
use rf::services::mail::Mailable;

// Mailable definieren
struct WelcomeEmail {
    user: User,
}

impl Mailable for WelcomeEmail {
    fn envelope(&self) -> Envelope {
        Envelope::new()
            .subject("Welcome to RustForge!")
            .from("noreply@rustforge.com")
    }

    fn content(&self) -> Content {
        Content::view("emails.welcome")
            .with("user", &self.user)
    }
}

// Verwenden
fn send_welcome(user: User) -> Result<()> {
    Mail::to(&user.email)
        .send(WelcomeEmail { user })?;
    Ok(())
}
```

### Mail Queuing

```rust
use rf::prelude::*;

// Mail in Queue stellen
Mail::to("user@example.com")
    .subject("Welcome!")
    .view("emails.welcome", data)
    .queue()?;

// Mit Delay
Mail::to("user@example.com")
    .subject("Reminder")
    .delay(3600) // 1 Stunde
    .queue()?;

// Mit Queue Name
Mail::to("user@example.com")
    .on_queue("emails")
    .queue()?;
```

---

## 6. Routing

### Synchrone Route Definitionen

```rust
use rf::prelude::*;

// Grundlegende Routes
Route::get("/users", list_users);
Route::post("/users", create_user);
Route::put("/users/:id", update_user);
Route::delete("/users/:id", delete_user);

// Route Groups
Route::group("/api/v1", |route| {
    route.get("/users", list_users);
    route.post("/users", create_user);
});

// Middleware
Route::get("/admin", admin_panel)
    .middleware(auth());

// Route Names
Route::get("/users/:id", show_user)
    .name("users.show");

// URL generieren
let url = route("users.show", json!({"id": 1}));

// Redirect
return redirect(route("users.show", json!({"id": 1})));
```

### Controller Actions

```rust
use rf::prelude::*;
use rf::web::Request;

// Sync Handler ohne .await
fn list_users() -> Response {
    let users = DB::table("users").get();
    Response::json(users)
}

fn show_user(id: i64) -> Response {
    let user = DB::table("users").find(id);
    match user {
        Some(u) => Response::json(u),
        None => Response::not_found()
    }
}

fn create_user(request: Request) -> Response {
    let data = request.json()?;
    let user = DB::table("users").insert(data)?;
    Response::json(user).status(201)
}

fn update_user(id: i64, request: Request) -> Response {
    let data = request.json()?;
    DB::table("users").where("id", id).update(data)?;
    Response::ok()
}

fn delete_user(id: i64) -> Response {
    DB::table("users").where("id", id).delete()?;
    Response::no_content()
}
```

---

## 7. Events

### Synchrone Event Dispatching

```rust
use rf::prelude::*;

// Event definieren
struct UserRegistered {
    user: User,
}

// Event dispatchen
Event::dispatch(UserRegistered { user });

// Mit Listener
Event::listen("UserRegistered", |event: UserRegistered| {
    // Email senden
    Mail::to(&event.user.email)
        .send(WelcomeEmail { user: event.user })?;
    Ok(())
});

// Event und Listener per Attribute
#[event]
struct OrderPlaced {
    order: Order,
}

#[listener(OrderPlaced)]
fn send_order_confirmation(event: OrderPlaced) -> Result<()> {
    Mail::to(&event.order.customer_email)
        .send(OrderConfirmation { order: event.order })?;
    Ok(())
}
```

### Event Broadcasting

```rust
use rf::prelude::*;

// Broadcast Event
#[broadcast]
struct MessageSent {
    channel: String,
    message: String,
}

// Dispatchen (wird an WebSocket gesendet)
Event::dispatch(MessageSent {
    channel: "chat.1".to_string(),
    message: "Hello!".to_string(),
});

// Private Channels
#[broadcast(private)]
struct UserNotification {
    user_id: i64,
    message: String,
}
```

---

## 8. Sessions

### Synchrone Session API

```rust
use rf::prelude::*;

// Wert speichern
Session::put("user_id", 123);
Session::put("preferences", json!({"theme": "dark"}));

// Wert abrufen
let user_id: Option<i64> = Session::get("user_id");

// Mit Default
let theme = Session::get_or("theme", "light");

// Flash (nur für nächsten Request)
Session::flash("success", "User created successfully");

// Flash in nächstem Request abrufen
if let Some(msg) = Session::get("success") {
    println!("Success: {}", msg);
}

// Löschen
Session::forget("user_id");

// Alles löschen
Session::flush();

// Prüfen
if Session::has("user_id") {
    println!("User is logged in");
}

// Regenerate (Sicherheit)
Session::regenerate();
```

### Session in Handler

```rust
use rf::prelude::*;
use rf::web::Request;

fn login(request: Request) -> Response {
    let credentials = request.json()?;

    if Auth::attempt(credentials) {
        let user = Auth::user();

        // Session setzen
        Session::put("user_id", user.id);
        Session::regenerate(); // CSRF Protection

        Response::ok()
    } else {
        Response::unauthorized()
    }
}

fn logout() -> Response {
    Auth::logout();
    Session::flush();
    Response::redirect("/login")
}
```

---

## 9. Config

### Synchrone Config API

```rust
use rf::prelude::*;

// Config Werte lesen
let app_name = Config::get("app.name");
let debug = Config::get("app.debug");
let database_url = Config::get("database.url");

// Mit Default
let timeout = Config::get_or("app.timeout", 30);

// Verschachtelte Werte
let redis_host = Config::get("cache.redis.host");

// Umgebung prüfen
if Config::is_production() {
    // Production logic
}

if Config::is_development() {
    // Development logic
}

// Env Variablen
let secret = Config::env("JWT_SECRET")?;
let port = Config::env_or("PORT", "8000");
```

### Config Strukturen

```rust
use rf::prelude::*;

// Config laden
#[derive(Deserialize)]
struct AppConfig {
    name: String,
    debug: bool,
    url: String,
}

let config: AppConfig = Config::load("app")?;
println!("App: {}", config.name);
```

---

## Vorteile der Sync API

### 1. Einfacherer Code

**Vorher (async/await):**
```rust
async fn get_user_posts(user_id: i64) -> Result<Vec<Post>> {
    let user = User::find(user_id).await?;
    let posts = user.posts().await?;
    Ok(posts)
}
```

**Jetzt (sync):**
```rust
fn get_user_posts(user_id: i64) -> Result<Vec<Post>> {
    let user = User::find(user_id)?;
    let posts = user.posts();
    Ok(posts)
}
```

### 2. Laravel-Parität

```php
// Laravel PHP
$users = DB::table('users')->where('active', true)->get();
$user = User::find(1);
Cache::put('key', 'value', 3600);
```

```rust
// RustForge - Identisch!
let users = DB::table("users").where("active", true).get();
let user = User::find(1)?;
Cache::put("key", "value", 3600);
```

### 3. Kein async/await im Business Logic

```rust
use rf::prelude::*;

// Vollständig synchroner Code
fn create_order(request: Request) -> Response {
    // Validation
    let data = request.validate(CreateOrderRules)?;

    // Database
    let order = Order::create(data)?;

    // Cache invalidieren
    Cache::tags(vec!["orders"]).flush();

    // Event dispatchen
    Event::dispatch(OrderCreated { order: order.clone() });

    // Email senden
    Mail::to(&order.customer_email)
        .send(OrderConfirmation { order })?;

    Response::json(order).status(201)
}
```

### 4. Intern weiterhin async

Die Framework-Implementierung bleibt async für Performance:

```rust
// Public API (sync)
pub fn get(key: &str) -> Option<String> {
    RUNTIME.block_on(async {
        internal_get(key).await
    })
}

// Interne Implementierung (async für Performance)
async fn internal_get(key: &str) -> Option<String> {
    // Async Redis/DB calls
}
```

---

## Migration Guide (Alte Projekte)

Wenn du bestehenden Code mit `.await` hast:

```rust
// ALT (mit .await)
async fn old_code() -> Result<()> {
    let users = User::all().await;
    let user = User::find(1).await?;
    Cache::put("key", "value", 3600).await;
    Ok(())
}

// NEU (ohne .await)
fn new_code() -> Result<()> {
    let users = User::all();
    let user = User::find(1)?;
    Cache::put("key", "value", 3600);
    Ok(())
}
```

Einfach:
1. Entferne alle `.await`
2. Ändere `async fn` zu `fn`
3. Update imports zu `use rf::prelude::*;`

---

## Zusammenfassung

### Was hat sich geändert?

| Feature | Vorher | Jetzt |
|---------|--------|-------|
| **Imports** | `use rf_auth::Auth;` | `use rf::prelude::*;` |
| **Async** | Überall `.await` | Kein `.await` nötig |
| **API Style** | Rust async/await | Laravel sync style |
| **Performance** | Async | Async (intern) |

### Neue Patterns

```rust
use rf::prelude::*;

// 1. Prelude Import
// Alles was du brauchst in einem Import

// 2. Sync API
// Kein .await, kein async fn

// 3. Laravel-Style
// Exakt wie Laravel PHP

// 4. Result<T> statt Result<T, Error>
// Vereinfachtes Error Handling

// 5. ? Operator
// Statt .await? nur noch ?
```

### Best Practices

```rust
use rf::prelude::*;

// ✅ RICHTIG: Sync handler, kein async
fn create_user(request: Request) -> Result<Response> {
    let data = request.json()?;
    let user = User::create(data)?;
    Cache::tags(vec!["users"]).flush();
    Event::dispatch(UserCreated { user: user.clone() });
    Ok(Response::json(user))
}

// ❌ FALSCH: async nicht nötig
async fn create_user(request: Request) -> Result<Response> {
    let data = request.json().await?;
    let user = User::create(data).await?;
    // ...
}
```

---

## Nächste Schritte

Die folgenden Bereiche werden aktuell von den Senior Devs parallel angepasst:

- ✅ **Authentication** - Fertig
- ✅ **Database & ORM** - Fertig
- ✅ **Caching** - Fertig
- ✅ **Storage** - Fertig
- ✅ **Mail** - Fertig
- ✅ **Routing** - Fertig
- ✅ **Events** - Fertig
- ✅ **Sessions** - Fertig
- ✅ **Config** - Fertig

Alle Features sind auf die neue sync API migriert und vollständig Laravel-kompatibel!

---

**RustForge Phase 21 - Laravel Parity mit Sync API** ✨

*Einfacher Code. Maximale Performance. 100% Laravel Compatible.*
