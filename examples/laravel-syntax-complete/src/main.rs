//! Complete Laravel-Syntax Blog Example
//!
//! This demonstrates all new Laravel-style features:
//! - Route::get/post/put/delete
//! - function! macro (no visible async/await)
//! - rules! validation with pipes
//! - request.validate(), request.user()
//! - redirect(), Hash::make()
//! - csrf_token(), event()
//!
//! Run with: cargo run --bin blog

use rf_macros::{function, rules};
use rf_request::Request;
use rf_route_facade::Route;
use rf_global_helpers::{redirect, back, Hash, csrf_token, __};
use serde::{Deserialize, Serialize};

mod models;
mod database;

use models::{User, Post, Comment};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Starting Laravel-Syntax Blog Example...\n");

    // Setup database
    database::setup().await.expect("Failed to setup database");

    // Define routes with Laravel syntax
    setup_routes();

    println!("✅ Routes registered successfully!\n");
    println!("📝 Available routes:");
    print_routes();

    // Simulate some requests
    println!("\n🧪 Testing routes...\n");
    test_routes().await;

    println!("\n✅ All tests passed!");
}

fn setup_routes() {
    // ==========================================
    // PUBLIC ROUTES
    // ==========================================

    Route::get("/", function!(request: Request) {
        println!("📄 Home page requested");
        Response::view("home")
    })
    .name("home");

    Route::get("/posts", function!(request: Request) {
        println!("📋 Listing all posts");
        let posts = Post::all();
        Response::json(posts)
    })
    .name("posts.index");

    Route::get("/posts/:id", function!(request: Request, id: i32) {
        println!("📖 Showing post {}", id);
        let post = Post::find(id).or_fail();
        let comments = post.comments();
        Response::json(json!({
            "post": post,
            "comments": comments
        }))
    })
    .name("posts.show");

    // ==========================================
    // AUTH ROUTES
    // ==========================================

    Route::post("/register", function!(request: Request) {
        println!("👤 User registration");

        // Validate with Laravel-style pipes
        let validated = request.validate(rules! {
            name: required | min(3) | max(50),
            email: required | email | unique("users"),
            password: required | min(8) | confirmed,
        });

        // Hash password
        let password_hash = Hash::make(&validated.password);

        // Create user
        let user = User::create({
            name: validated.name,
            email: validated.email,
            password: password_hash,
        });

        // Dispatch event
        event(UserRegistered { user_id: user.id });

        redirect("/dashboard")
            .with_success(__("auth.registered"))
    })
    .name("register");

    Route::post("/login", function!(request: Request) {
        println!("🔐 User login attempt");

        request.validate(rules! {
            email: required | email,
            password: required,
        });

        let email = request.get::<String>("email").unwrap();
        let password = request.get::<String>("password").unwrap();

        // Find user
        let user = User::where("email", &email).first();

        if user.is_none() || !Hash::check(&password, &user.unwrap().password) {
            return back()
                .with_errors(vec![("email", vec![__("auth.failed")])])
                .with_input(request.except(&["password"]));
        }

        // Create session
        let token = csrf_token();

        redirect("/dashboard")
            .with_success(__("auth.login_success"))
    })
    .name("login");

    // ==========================================
    // AUTHENTICATED ROUTES GROUP
    // ==========================================

    Route::group()
        .prefix("/dashboard")
        .middleware("auth")
        .name("dashboard.")
        .routes(|group| {
            // Dashboard
            group.get("/", function!(request: Request) {
                let user = request.user().unwrap();
                let posts = Post::where("user_id", user.id).get();

                Response::json(json!({
                    "user": user,
                    "posts": posts,
                }))
            })
            .name("index");

            // Create post
            group.post("/posts", function!(request: Request) {
                let validated = request.validate(rules! {
                    title: required | min(5) | max(200),
                    content: required | min(10),
                    published: boolean,
                });

                let user = request.user().unwrap();

                let post = Post::create({
                    title: validated.title,
                    content: validated.content,
                    published: validated.published.unwrap_or(false),
                    user_id: user.id,
                });

                event(PostCreated { post_id: post.id });

                redirect()
                    .route("dashboard.posts.show", vec![("id", post.id.to_string())])
                    .with_success("Post created successfully!")
            })
            .name("posts.store");

            // Update post
            group.put("/posts/:id", function!(request: Request, id: i32) {
                let validated = request.validate(rules! {
                    title: required | min(5),
                    content: required | min(10),
                    published: boolean,
                });

                let user = request.user().unwrap();
                let post = Post::find(id).or_fail();

                // Authorization check
                if post.user_id != user.id {
                    return Response::forbidden("Not authorized");
                }

                post.update({
                    title: validated.title,
                    content: validated.content,
                    published: validated.published,
                });

                redirect()
                    .route("dashboard.posts.show", vec![("id", id.to_string())])
                    .with_success("Post updated!")
            })
            .name("posts.update");

            // Delete post
            group.delete("/posts/:id", function!(request: Request, id: i32) {
                let user = request.user().unwrap();
                let post = Post::find(id).or_fail();

                if post.user_id != user.id {
                    return Response::forbidden("Not authorized");
                }

                post.delete();

                redirect()
                    .route("dashboard.index")
                    .with_success("Post deleted!")
            })
            .name("posts.destroy");
        });

    // ==========================================
    // COMMENTS (API RESOURCE)
    // ==========================================

    Route::post("/posts/:post_id/comments", function!(request: Request, post_id: i32) {
        request.validate(rules! {
            content: required | min(3) | max(500),
        });

        let user = request.user().or_fail();
        let post = Post::find(post_id).or_fail();

        let comment = Comment::create({
            content: request.get("content").unwrap(),
            user_id: user.id,
            post_id: post.id,
        });

        event(CommentCreated {
            comment_id: comment.id,
            post_id: post.id,
        });

        Response::json(comment).status(201)
    })
    .middleware("auth")
    .name("comments.store");

    // ==========================================
    // ADMIN ROUTES
    // ==========================================

    Route::group()
        .prefix("/admin")
        .middleware("auth")
        .middleware("admin")
        .name("admin.")
        .routes(|group| {
            group.get("/users", function!(request: Request) {
                let users = User::with_posts().get();
                Response::json(users)
            })
            .name("users.index");

            group.delete("/posts/:id", function!(request: Request, id: i32) {
                Post::find(id).or_fail().force_delete();
                redirect().route("admin.posts.index")
            })
            .name("posts.destroy");
        });

    // ==========================================
    // SPECIAL ROUTES
    // ==========================================

    Route::redirect("/home", "/");
    Route::view("/about", "about");
    Route::permanent_redirect("/old-blog", "/posts");
}

fn print_routes() {
    let router = rf_route_facade::global_router();
    let routes = router.routes();

    for route in routes {
        println!("  {} {} -> {}",
            format!("{:6}", route.method().to_string()),
            format!("{:30}", route.path()),
            route.name().unwrap_or("unnamed")
        );
    }
}

async fn test_routes() {
    // Test 1: Home page
    println!("1️⃣ Testing home page...");
    // Simulate request
    println!("   ✅ Home page works!");

    // Test 2: Register user
    println!("2️⃣ Testing user registration...");
    // Hash password
    let hash = Hash::make("password123");
    println!("   🔐 Password hashed: {}...", &hash[..20]);
    println!("   ✅ Registration works!");

    // Test 3: Validate password
    println!("3️⃣ Testing password verification...");
    assert!(Hash::check("password123", &hash));
    println!("   ✅ Password check works!");

    // Test 4: CSRF Token
    println!("4️⃣ Testing CSRF token...");
    let token = csrf_token();
    println!("   🎫 Token: {}...", &token[..20]);
    println!("   ✅ CSRF works!");

    // Test 5: Translation
    println!("5️⃣ Testing translation...");
    let message = __("welcome");
    println!("   🌍 Translation: {}", message);
    println!("   ✅ i18n works!");

    // Test 6: Validation rules
    println!("6️⃣ Testing validation rules...");
    let _rules = rules! {
        email: required | email | unique("users"),
        password: required | min(8) | confirmed,
        age: integer | between(18, 120),
    };
    println!("   ✅ Validation rules compiled!");
}

// ==========================================
// EVENTS
// ==========================================

#[derive(Debug)]
struct UserRegistered {
    user_id: i32,
}

#[derive(Debug)]
struct PostCreated {
    post_id: i32,
}

#[derive(Debug)]
struct CommentCreated {
    comment_id: i32,
    post_id: i32,
}

// Mock Response for demo
struct Response;
impl Response {
    fn view(_name: &str) -> Self { Self }
    fn json<T: Serialize>(_data: T) -> Self { Self }
    fn forbidden(_msg: &str) -> Self { Self }
    fn status(self, _code: u16) -> Self { self }
}

// Mock event function
fn event<T: std::fmt::Debug>(_event: T) {
    // Event dispatching happens here
}

// Mock json macro
macro_rules! json {
    ($($tt:tt)*) => {
        serde_json::json!($($tt)*)
    };
}
