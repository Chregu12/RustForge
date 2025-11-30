# RustForge React Starter Kit

A modern React starter kit for RustForge applications with TypeScript, Tailwind CSS, and shadcn/ui components.

## Features

- **React 18** with TypeScript
- **Vite** for fast development and builds
- **Tailwind CSS** for styling
- **shadcn/ui** components (Radix UI primitives)
- **React Router** for client-side routing
- **Axios** for API requests
- **Authentication** - Login, Register, Password Reset
- **Protected Routes** with auth guards
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

The app will be available at `http://localhost:3001`.

## Project Structure

```
src/
├── components/
│   └── ui/          # shadcn/ui components
├── contexts/
│   └── AuthContext  # Authentication state
├── layouts/
│   ├── AppLayout    # Authenticated layout
│   └── GuestLayout  # Guest layout
├── lib/
│   ├── api.ts       # API client & auth functions
│   └── utils.ts     # Utility functions
├── pages/
│   ├── auth/        # Auth pages (Login, Register, etc.)
│   ├── Dashboard    # Main dashboard
│   └── Profile      # User profile
├── App.tsx          # Routes configuration
├── main.tsx         # Entry point
└── index.css        # Tailwind styles
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

## Customization

### Adding shadcn/ui Components

This starter includes basic components. Add more from [shadcn/ui](https://ui.shadcn.com/):

```bash
npx shadcn-ui@latest add [component-name]
```

### Theming

Customize colors in `src/index.css` using CSS variables.

## License

MIT
