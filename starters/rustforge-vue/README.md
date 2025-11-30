# RustForge Vue Starter Kit

A modern Vue 3 starter kit for RustForge applications with TypeScript, Tailwind CSS, and Pinia state management.

## Features

- **Vue 3** with Composition API and TypeScript
- **Vite** for fast development and builds
- **Tailwind CSS** for styling
- **Pinia** for state management
- **Vue Router** for client-side routing
- **Axios** for API requests
- **Authentication** - Login, Register, Password Reset
- **Protected Routes** with navigation guards
- **Dark Mode** support

## Prerequisites

- Node.js 18+
- RustForge backend running on `http://localhost:3000`

## Quick Start

```bash
# Install dependencies
npm install

# Start development server
npm run dev
```

The app will be available at `http://localhost:3002`.

## Project Structure

```
src/
├── assets/
│   └── main.css      # Tailwind styles
├── components/
│   └── ui/           # UI components
├── layouts/
│   ├── AppLayout     # Authenticated layout
│   └── GuestLayout   # Guest layout
├── lib/
│   ├── api.ts        # API client & auth functions
│   └── utils.ts      # Utility functions
├── router/
│   └── index.ts      # Route definitions
├── stores/
│   └── auth.ts       # Auth state (Pinia)
├── views/
│   ├── auth/         # Auth views (Login, Register, etc.)
│   ├── DashboardView # Main dashboard
│   └── ProfileView   # User profile
├── App.vue           # Root component
└── main.ts           # Entry point
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

### Environment Variables

Create a `.env` file:

```env
VITE_API_URL=http://localhost:3000/api
```

### Proxy Configuration

The development server proxies `/api` requests to `http://localhost:3000`. Configure this in `vite.config.ts`.

## Building for Production

```bash
npm run build
```

Output will be in the `dist/` directory.

## License

MIT
