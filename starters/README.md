# RustForge Starter Kits

Official frontend starter kits for RustForge applications. These starter kits provide pre-built authentication scaffolding similar to Laravel's Breeze/Jetstream.

## Available Starter Kits

| Starter Kit | Framework | Port | Description |
|-------------|-----------|------|-------------|
| [rustforge-react](./rustforge-react) | React 18 | 3001 | React + TypeScript + Vite + Tailwind + shadcn/ui |
| [rustforge-vue](./rustforge-vue) | Vue 3 | 3002 | Vue 3 + TypeScript + Vite + Tailwind + Pinia |
| [rustforge-angular](./rustforge-angular) | Angular 18 | 3003 | Angular 18 + TypeScript + Tailwind + Signals |
| [backend-api-example](./backend-api-example) | Rust | 3000 | Demo backend API for testing starter kits |

## Features

All starter kits include:

- **Authentication Pages**
  - Login
  - Registration
  - Forgot Password
  - Reset Password
  - Profile Management

- **Protected Routes**
  - Dashboard (authenticated)
  - Profile (authenticated)

- **Modern Stack**
  - TypeScript
  - Tailwind CSS
  - Dark Mode support
  - Responsive design

## Quick Start

1. **Start the demo backend** (or your own RustForge backend):
   ```bash
   cd starters/backend-api-example
   cargo run
   # Demo: email=demo@example.com, password=any
   ```

2. **Choose a starter kit** and navigate to its directory:
   ```bash
   cd starters/rustforge-react   # or rustforge-vue, rustforge-angular
   ```

3. **Install dependencies**:
   ```bash
   npm install
   ```

4. **Start the development server**:
   ```bash
   npm run dev   # React/Vue
   npm start     # Angular
   ```

## Backend API Requirements

Your RustForge backend should implement these API endpoints:

```
POST   /api/auth/login           # Login with email/password
POST   /api/auth/register        # Create new account
POST   /api/auth/logout          # Logout (invalidate token)
POST   /api/auth/forgot-password # Request password reset email
POST   /api/auth/reset-password  # Reset password with token
GET    /api/user                 # Get current authenticated user
PUT    /api/user/profile         # Update profile (name, email)
PUT    /api/user/password        # Update password
DELETE /api/user                 # Delete account
```

### Example Backend Implementation

Using RustForge with `rf-sanctum`:

```rust
use rustforge::prelude::*;
use rf_sanctum::prelude::*;

// Authentication routes
Route::post("/api/auth/login", auth::login);
Route::post("/api/auth/register", auth::register);
Route::post("/api/auth/logout", auth::logout).middleware(Sanctum::auth());
Route::post("/api/auth/forgot-password", auth::forgot_password);
Route::post("/api/auth/reset-password", auth::reset_password);

// User routes (protected)
Route::get("/api/user", user::show).middleware(Sanctum::auth());
Route::put("/api/user/profile", user::update_profile).middleware(Sanctum::auth());
Route::put("/api/user/password", user::update_password).middleware(Sanctum::auth());
Route::delete("/api/user", user::destroy).middleware(Sanctum::auth());
```

## Customization

Each starter kit can be customized:

- **Tailwind Theme**: Edit `tailwind.config.js` and CSS variables in the styles file
- **Components**: Add or modify components in the `components/` directory
- **Routes**: Add new routes and pages as needed
- **API Client**: Configure the base URL and interceptors in `lib/api.ts` (or equivalent)

## License

MIT
