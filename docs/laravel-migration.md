# Laravel to RustForge Migration Guide

This guide helps Laravel developers transition to RustForge by comparing familiar Laravel concepts with their RustForge equivalents.

## Philosophy

RustForge aims to maintain Laravel's elegant API while leveraging Rust's performance and type safety. The main differences you'll notice:

1. **Async/Await** - Most operations are asynchronous
2. **Type Safety** - Rust's compiler catches errors at compile time
3. **Error Handling** - Explicit `Result<T, E>` types instead of exceptions
4. **No Magic** - More explicit, less reflection-based behavior

## Project Structure

### Laravel
```
app/
├── Http/Controllers/
├── Models/
├── Providers/
config/
routes/
resources/views/
database/migrations/
```

### RustForge
```
app/
├── Http/Controllers/
├── Models/
├── Services/
config/
routes/
resources/views/
database/migrations/
```

Almost identical! RustForge follows Laravel's familiar structure.

## Routing

### Laravel
```php
Route::get('/users', [UserController::class, 'index']);
Route::post('/users', [UserController::class, 'store']);
Route::resource('posts', PostController::class);
```

### RustForge
```rust
Router::new()
    .route("/users", Route::get(UserController::index))
    .route("/users", Route::post(UserController::store))
    .resource("posts", PostController::resource())
```

## Controllers

### Laravel
```php
class UserController extends Controller
{
    public function index()
    {
        $users = User::with('posts')->get();
        return UserResource::collection($users);
    }
}
```

### RustForge
```rust
pub struct UserController;

impl UserController {
    pub async fn index() -> Result<Response> {
        let users = User::with("posts").get().await?;
        Ok(Response::json(UserResource::collection(users)))
    }
}
```

## Models & ORM

### Laravel
```php
class User extends Model
{
    protected $fillable = ['name', 'email'];

    public function posts()
    {
        return $this->hasMany(Post::class);
    }
}

// Usage
$user = User::find(1);
$posts = $user->posts;
```

### RustForge
```rust
#[derive(Model)]
#[table_name = "users"]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

impl User {
    pub fn posts(&self) -> HasMany<Post> {
        self.has_many()
    }
}

// Usage
let user = User::find(1).await?;
let posts = user.posts().get().await?;
```

## Queries

### Laravel
```php
// Simple queries
$users = User::where('active', true)->get();
$user = User::find(1);

// Relationships
$users = User::with('posts', 'comments')->get();

// Aggregates
$count = User::where('active', true)->count();
```

### RustForge
```rust
// Simple queries
let users = User::where_column("active", true).get().await?;
let user = User::find(1).await?;

// Relationships
let users = User::with(&["posts", "comments"]).get().await?;

// Aggregates
let count = User::where_column("active", true).count().await?;
```

## Migrations

### Laravel
```php
Schema::create('users', function (Blueprint $table) {
    $table->id();
    $table->string('name');
    $table->string('email')->unique();
    $table->timestamps();
});
```

### RustForge
```rust
fn up(&self) -> Migration {
    Migration::create_table("users", |table| {
        table.id();
        table.string("name");
        table.string("email").unique();
        table.timestamps();
    })
}
```

## Validation

### Laravel
```php
$request->validate([
    'name' => 'required|string|max:255',
    'email' => 'required|email|unique:users',
    'age' => 'required|integer|min:18',
]);
```

### RustForge
```rust
let validator = Validator::new()
    .rule("name", required().string().max(255))
    .rule("email", required().email().unique("users"))
    .rule("age", required().integer().min(18));

let validated = validator.validate(data).await?;
```

## Authentication

### Laravel
```php
// Login
Auth::attempt(['email' => $email, 'password' => $password]);

// Get user
$user = Auth::user();

// Logout
Auth::logout();
```

### RustForge
```rust
// Login
Auth::attempt(credentials).await?;

// Get user (in handler)
async fn profile(auth: Authenticated) -> Result<Response> {
    let user = auth.user;
    Ok(Response::json(user))
}

// Logout
Auth::logout().await?;
```

## Middleware

### Laravel
```php
class CheckAge
{
    public function handle($request, Closure $next)
    {
        if ($request->age < 18) {
            return redirect('home');
        }
        return $next($request);
    }
}
```

### RustForge
```rust
async fn check_age(req: Request, next: Next) -> Result<Response> {
    if req.input::<i32>("age")? < 18 {
        return Ok(Response::redirect("/home"));
    }
    next.run(req).await
}
```

## Queues & Jobs

### Laravel
```php
class SendEmail implements ShouldQueue
{
    public function handle()
    {
        Mail::to($this->user)->send(new Welcome);
    }
}

// Dispatch
SendEmail::dispatch($user);
```

### RustForge
```rust
#[derive(Job)]
pub struct SendEmail {
    user_id: i64,
}

impl Handler for SendEmail {
    async fn handle(&self) -> Result<()> {
        let user = User::find(self.user_id).await?;
        Mail::to(&user).send(Welcome::new()).await?;
        Ok(())
    }
}

// Dispatch
SendEmail { user_id: 1 }.dispatch().await?;
```

## Collections

### Laravel
```php
$collection = collect([1, 2, 3, 4, 5]);
$filtered = $collection->filter(fn($x) => $x > 2);
$mapped = $filtered->map(fn($x) => $x * 2);
```

### RustForge
```rust
let collection = Collection::from(vec![1, 2, 3, 4, 5]);
let filtered = collection.filter(|x| *x > 2);
let mapped = filtered.map(|x| x * 2);
```

## API Resources

### Laravel
```php
class UserResource extends JsonResource
{
    public function toArray($request)
    {
        return [
            'id' => $this->id,
            'name' => $this->name,
            'email' => $this->email,
        ];
    }
}
```

### RustForge
```rust
#[derive(Resource)]
pub struct UserResource {
    pub id: i64,
    pub name: String,
    pub email: String,
}

impl From<User> for UserResource {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
        }
    }
}
```

## Key Differences

### 1. Async/Await

Laravel:
```php
$users = User::all();
```

RustForge:
```rust
let users = User::all().await?;
```

### 2. Error Handling

Laravel uses exceptions:
```php
try {
    $user = User::findOrFail($id);
} catch (ModelNotFoundException $e) {
    abort(404);
}
```

RustForge uses Result:
```rust
let user = User::find(id).await?;
// Or explicit handling
match User::find(id).await {
    Ok(user) => Ok(Response::json(user)),
    Err(_) => Err(Error::NotFound),
}
```

### 3. Type Safety

Laravel (dynamic):
```php
$user->name = 123; // Works, but wrong type
```

RustForge (static):
```rust
user.name = 123; // Compile error!
user.name = "John".to_string(); // Correct
```

## Migration Checklist

- [ ] Install Rust and Forge CLI
- [ ] Create new RustForge project
- [ ] Port database schema (migrations)
- [ ] Convert models
- [ ] Port controllers
- [ ] Update routes
- [ ] Convert middleware
- [ ] Port jobs and listeners
- [ ] Update tests
- [ ] Deploy!

## Getting Help

- [RustForge Documentation](https://rustforge.dev/docs)
- [Laravel Comparison Guide](../LARAVEL_COMPARISON_ANALYSIS_UPDATED.md)
- [Discord Community](https://discord.gg/rustforge)
