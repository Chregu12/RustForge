//! Web framework integration example
//!
//! This example shows how to integrate rf-socialite with a web framework.
//! This is a conceptual example showing the flow.

use rf_socialite::*;
use rf_socialite::routes::*;

/// Example web application state
struct AppState {
    socialite: SocialiteManager,
}

impl AppState {
    fn new() -> Self {
        // Load configuration from environment
        let config = SocialiteConfig::from_env();
        let socialite = SocialiteManager::new(config);

        Self { socialite }
    }
}

fn main() {
    println!("=== Web Framework Integration Example ===\n");

    // This is a conceptual example showing the routes you would create
    println!("Routes to implement in your web framework:\n");

    println!("GET  /auth/{{provider}}           -> redirect_to_provider");
    println!("GET  /auth/{{provider}}/callback  -> handle_oauth_callback");
    println!();

    println!("Example route handlers:\n");
    print_example_code();
}

fn print_example_code() {
    let code = r#"
// Route 1: Redirect to OAuth provider
async fn redirect_to_provider(
    provider: String,
    state: AppState,
) -> Result<Redirect, Error> {
    // Generate auth URL with state and PKCE
    let auth_url = routes::redirect_to_provider(
        &state.socialite,
        &provider,
        true, // use PKCE
    )?;

    Ok(Redirect::to(&auth_url))
}

// Route 2: Handle OAuth callback
async fn handle_oauth_callback(
    provider: String,
    params: CallbackParams,
    state: AppState,
) -> Result<Redirect, Error> {
    // Handle the callback
    let user = routes::handle_callback(
        &state.socialite,
        &provider,
        params,
    ).await?;

    // Find or create user in database
    let local_user = find_or_create_user(&user).await?;

    // Create session / log in user
    create_session(&local_user);

    // Redirect to dashboard
    Ok(Redirect::to("/dashboard"))
}

// Helper: Find or create user
async fn find_or_create_user(social_user: &User) -> Result<LocalUser, Error> {
    // Check if social account exists
    if let Some(account) = SocialAccount::find_by_provider(
        &social_user.provider,
        &social_user.id,
    ).await? {
        return LocalUser::find(account.user_id).await;
    }

    // Check if user with email exists
    if let Some(email) = &social_user.email {
        if let Some(user) = LocalUser::find_by_email(email).await? {
            // Link social account to existing user
            link_social_account(&user, social_user).await?;
            return Ok(user);
        }
    }

    // Create new user
    let user = LocalUser::create_from_social(social_user).await?;
    link_social_account(&user, social_user).await?;

    Ok(user)
}

// Helper: Link social account
async fn link_social_account(
    user: &LocalUser,
    social_user: &User,
) -> Result<(), Error> {
    let account = SocialAccount::new(
        user.id,
        &social_user.provider,
        &social_user.id,
        &social_user.token,
    );

    account.save().await?;
    Ok(())
}

// Database models (pseudo-code)
struct LocalUser {
    id: i64,
    name: String,
    email: String,
    avatar: Option<String>,
}

impl LocalUser {
    async fn find(id: i64) -> Result<Self, Error> {
        // Query database
        todo!()
    }

    async fn find_by_email(email: &str) -> Result<Option<Self>, Error> {
        // Query database
        todo!()
    }

    async fn create_from_social(social: &User) -> Result<Self, Error> {
        // Insert into database
        todo!()
    }
}

// Example with Axum framework
use axum::{
    Router,
    routing::get,
    extract::{Path, Query, State},
    response::Redirect,
};

fn create_routes() -> Router {
    Router::new()
        .route("/auth/:provider", get(auth_redirect))
        .route("/auth/:provider/callback", get(auth_callback))
        .with_state(AppState::new())
}

async fn auth_redirect(
    Path(provider): Path<String>,
    State(app): State<AppState>,
) -> Result<Redirect, Error> {
    let url = routes::redirect_to_provider(&app.socialite, &provider, true)?;
    Ok(Redirect::to(&url))
}

async fn auth_callback(
    Path(provider): Path<String>,
    Query(params): Query<CallbackParams>,
    State(app): State<AppState>,
) -> Result<Redirect, Error> {
    let user = routes::handle_callback(&app.socialite, &provider, params).await?;
    let local_user = find_or_create_user(&user).await?;
    create_session(&local_user);
    Ok(Redirect::to("/dashboard"))
}
"#;

    println!("{}", code);
}

/// Example: Multiple provider support
#[allow(dead_code)]
fn multiple_providers_example() {
    let code = r#"
// Support multiple providers simultaneously
let config = SocialiteConfig::new()
    .with_google(ProviderConfig::new(
        env::var("GOOGLE_CLIENT_ID")?,
        env::var("GOOGLE_CLIENT_SECRET")?,
        "http://localhost:8000/auth/google/callback",
    ))
    .with_github(ProviderConfig::new(
        env::var("GITHUB_CLIENT_ID")?,
        env::var("GITHUB_CLIENT_SECRET")?,
        "http://localhost:8000/auth/github/callback",
    ))
    .with_facebook(ProviderConfig::new(
        env::var("FACEBOOK_CLIENT_ID")?,
        env::var("FACEBOOK_CLIENT_SECRET")?,
        "http://localhost:8000/auth/facebook/callback",
    ));

let manager = SocialiteManager::new(config);

// Users can choose which provider to use
// GET /auth/google
// GET /auth/github
// GET /auth/facebook
"#;

    println!("Multiple Providers:\n{}", code);
}

/// Example: Account linking UI
#[allow(dead_code)]
fn account_linking_ui_example() {
    let code = r#"
// When a social login matches an existing email:

async fn handle_callback_with_linking(
    provider: String,
    params: CallbackParams,
    state: AppState,
) -> Result<Response, Error> {
    let social_user = routes::handle_callback(&state.socialite, &provider, params).await?;

    // Check linking strategy
    let linker = AccountLinker::new(LinkingStrategy::AskUser);

    if linker.should_ask_user() {
        // Check if email exists
        if let Some(email) = &social_user.email {
            if let Some(existing_user) = LocalUser::find_by_email(email).await? {
                // Show linking confirmation page
                return Ok(render_linking_page(existing_user, social_user));
            }
        }
    }

    // Auto-link or create new user
    let user = find_or_create_user(&social_user).await?;
    create_session(&user);
    Ok(Redirect::to("/dashboard"))
}

// Template for linking confirmation
fn render_linking_page(existing_user: LocalUser, social_user: User) -> Response {
    // "We found an account with email {email}.
    //  Would you like to link your {provider} account to it?"
    //
    // [Link Account] [Create New Account]
    todo!()
}
"#;

    println!("Account Linking UI:\n{}", code);
}

/// Example: Token refresh
#[allow(dead_code)]
fn token_refresh_example() {
    let code = r#"
// Automatically refresh expired tokens
async fn get_valid_token(account: &SocialAccount) -> Result<String, Error> {
    if !account.needs_refresh() {
        return Ok(account.access_token.clone());
    }

    // Token needs refresh
    if let Some(refresh_token) = &account.refresh_token {
        let manager = SocialiteManager::from_env();
        let driver = manager.driver(&account.provider)?.build()?;

        let new_token = driver.refresh_token(refresh_token).await?;

        // Update database
        account.update_token(
            &new_token.access_token,
            new_token.refresh_token.as_deref(),
            new_token.expires_in,
        ).await?;

        return Ok(new_token.access_token);
    }

    Err(Error::TokenExpired)
}
"#;

    println!("Token Refresh:\n{}", code);
}
