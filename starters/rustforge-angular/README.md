# RustForge Angular Starter Kit

A modern Angular 18 starter kit for RustForge applications with TypeScript, Tailwind CSS, and standalone components.

## Features

- **Angular 18** with standalone components
- **Angular CLI** for development and builds
- **Tailwind CSS** for styling
- **Angular Router** for client-side routing
- **Signals** for reactive state management
- **HTTP Interceptors** for auth token handling
- **Authentication** - Login, Register, Password Reset
- **Route Guards** for protected routes
- **Dark Mode** support

## Prerequisites

- Node.js 18+
- Angular CLI (`npm install -g @angular/cli`)
- RustForge backend running on `http://localhost:3000`

## Quick Start

```bash
# Install dependencies
npm install

# Start development server
npm start
```

The app will be available at `http://localhost:3003`.

## Project Structure

```
src/
├── app/
│   ├── guards/           # Route guards
│   ├── interceptors/     # HTTP interceptors
│   ├── layouts/          # Layout components
│   ├── pages/
│   │   ├── auth/         # Auth pages
│   │   ├── dashboard/    # Dashboard page
│   │   └── profile/      # Profile page
│   ├── services/         # Services (AuthService)
│   ├── app.component.ts  # Root component
│   └── app.routes.ts     # Route definitions
├── styles.css            # Tailwind styles
├── index.html            # HTML entry point
└── main.ts               # Bootstrap
```

## API Endpoints

The starter expects the following API endpoints on your RustForge backend:

```
POST   /api/auth/login           # Login
POST   /api/auth/register        # Register
POST   /api/auth/logout          # Logout
POST   /api/auth/forgot-password # Request password reset
POST   /api/auth/reset-password  # Reset password
GET    /api/user                 # Get current user
PUT    /api/user/profile         # Update profile
PUT    /api/user/password        # Update password
DELETE /api/user                 # Delete account
```

## Configuration

### Proxy Configuration

The development server proxies `/api` requests to `http://localhost:3000`. Configure this in `proxy.conf.json`.

## Building for Production

```bash
npm run build
```

Output will be in the `dist/` directory.

## Angular CLI Commands

```bash
# Generate a new component
ng generate component components/my-component

# Generate a new service
ng generate service services/my-service

# Run tests
ng test

# Lint
ng lint
```

## License

MIT
