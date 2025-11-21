//! Basic OAuth2 flow example
//!
//! This example demonstrates how to use rf-socialite for OAuth authentication.
//!
//! To run this example:
//! 1. Set environment variables:
//!    - GITHUB_CLIENT_ID=your-github-client-id
//!    - GITHUB_CLIENT_SECRET=your-github-client-secret
//!    - GITHUB_REDIRECT_URI=http://localhost:8000/auth/github/callback
//!
//! 2. Run: cargo run --example basic_oauth

use rf_socialite::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== RustForge Socialite - Basic OAuth Example ===\n");

    // Example 1: Simple driver usage
    simple_driver_example()?;

    // Example 2: Manager usage
    manager_example()?;

    // Example 3: PKCE flow
    pkce_example()?;

    // Example 4: State management
    state_management_example()?;

    // Example 5: Account linking
    account_linking_example();

    Ok(())
}

/// Example 1: Simple driver usage
fn simple_driver_example() -> Result<()> {
    println!("1. Simple Driver Example");
    println!("------------------------");

    let mut driver = Socialite::driver(Provider::GitHub)
        .client_id("your-client-id")
        .client_secret("your-client-secret")
        .redirect_url("http://localhost:8000/auth/github/callback")
        .build()?;

    let auth_url = driver.redirect()?;
    println!("Authorization URL: {}\n", auth_url);

    Ok(())
}

/// Example 2: Manager usage
fn manager_example() -> Result<()> {
    println!("2. Manager Example");
    println!("------------------");

    // Create configuration
    let config = SocialiteConfig::new()
        .with_github(ProviderConfig::new(
            "github-client-id",
            "github-client-secret",
            "http://localhost:8000/auth/github/callback",
        ))
        .with_google(ProviderConfig::new(
            "google-client-id",
            "google-client-secret",
            "http://localhost:8000/auth/google/callback",
        ));

    // Create manager
    let manager = SocialiteManager::new(config);

    // Generate state for CSRF protection
    let state = manager.generate_state();
    println!("Generated state token: {}", state);

    // Get driver with state
    let mut driver = manager.github()?
        .state(state.clone())
        .build()?;

    let auth_url = driver.redirect()?;
    println!("GitHub auth URL: {}\n", auth_url);

    Ok(())
}

/// Example 3: PKCE flow
fn pkce_example() -> Result<()> {
    println!("3. PKCE Flow Example");
    println!("--------------------");

    // PKCE is recommended for public clients (mobile apps, SPAs)
    let mut driver = Socialite::driver(Provider::Google)
        .client_id("google-client-id")
        .client_secret("google-client-secret")
        .redirect_url("http://localhost:8000/auth/google/callback")
        .with_pkce()
        .build()?;

    let auth_url = driver.redirect()?;
    println!("Google auth URL (with PKCE): {}\n", auth_url);

    // The PKCE code verifier is stored in the driver and will be
    // automatically used when exchanging the authorization code

    Ok(())
}

/// Example 4: State management
fn state_management_example() -> Result<()> {
    println!("4. State Management Example");
    println!("---------------------------");

    let state_manager = StateManager::new();

    // Generate and verify state
    let state = state_manager.generate();
    println!("Generated state: {}", state);
    println!("State is valid: {}", state_manager.verify(&state));
    println!("State is valid (2nd try): {}\n", state_manager.verify(&state));

    Ok(())
}

/// Example 5: Account linking
fn account_linking_example() {
    println!("5. Account Linking Example");
    println!("--------------------------");

    // Create a social account
    let social_account = SocialAccount::new(
        1, // user_id
        "github",
        "12345", // provider_user_id
        "access-token-abc",
    )
    .with_refresh_token("refresh-token-xyz");

    println!("Social account created:");
    println!("  Provider: {}", social_account.provider);
    println!("  Provider User ID: {}", social_account.provider_user_id);
    println!("  User ID: {}", social_account.user_id);
    println!("  Has refresh token: {}\n", social_account.refresh_token.is_some());

    // Configure linking strategy
    let linker = AccountLinker::new(LinkingStrategy::AutoLinkByEmail);
    println!("Linking strategy: AutoLinkByEmail");
    println!("  Should auto-link: {}", linker.should_auto_link());
    println!("  Should create new: {}", linker.should_create_new());
    println!();
}

/// Example 6: Complete OAuth flow (conceptual)
#[allow(dead_code)]
async fn complete_oauth_flow_example() -> Result<()> {
    println!("6. Complete OAuth Flow");
    println!("----------------------");

    // Initialize manager
    let manager = SocialiteManager::from_env();

    // Step 1: Redirect to provider
    let state = manager.generate_state();
    let mut driver = manager.github()?
        .state(state.clone())
        .with_pkce()
        .build()?;

    let auth_url = driver.redirect()?;
    println!("1. Redirect user to: {}", auth_url);

    // Step 2: User authorizes and is redirected back with code
    // (This would come from your web framework)
    let code = "authorization-code-from-callback";
    let received_state = state.clone();

    // Step 3: Verify state
    if !manager.verify_state(&received_state) {
        println!("ERROR: Invalid state!");
        return Ok(());
    }

    // Step 4: Exchange code for user info
    let user = driver.user_from_code(code).await?;
    println!("2. User authenticated:");
    println!("   ID: {}", user.id);
    println!("   Name: {}", user.name);
    println!("   Email: {:?}", user.email);
    println!("   Provider: {}", user.provider);

    // Step 5: Find or create user in database
    // let local_user = find_or_create_user(&user).await?;

    // Step 6: Log in the user
    // login_user(&local_user);

    Ok(())
}

/// Example helper: Find or create user
#[allow(dead_code)]
async fn find_or_create_user(social_user: &User) -> anyhow::Result<i64> {
    // This is a conceptual example - you would implement this with your database

    // 1. Check if social account exists
    // let social_account = db.find_social_account(&social_user.provider, &social_user.id).await?;

    // 2. If exists, return the user
    // if let Some(account) = social_account {
    //     return Ok(account.user_id);
    // }

    // 3. Check if user with email exists
    // if let Some(email) = &social_user.email {
    //     if let Some(user) = db.find_user_by_email(email).await? {
    //         // Link social account to existing user
    //         db.create_social_account(user.id, social_user).await?;
    //         return Ok(user.id);
    //     }
    // }

    // 4. Create new user
    // let user_id = db.create_user(social_user).await?;
    // db.create_social_account(user_id, social_user).await?;

    // For demo purposes:
    println!("Would create/find user for: {}", social_user.name);
    Ok(1)
}
