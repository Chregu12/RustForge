# RustForge Backend API Example

A minimal example backend that demonstrates the API endpoints required by the RustForge starter kits.

## Quick Start

```bash
# Run the server
cargo run

# Server starts on http://localhost:3000
```

## Demo Credentials

- **Email**: `demo@example.com`
- **Password**: any password (demo mode accepts all)

## Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/auth/login` | Authenticate user |
| POST | `/api/auth/register` | Create new account |
| POST | `/api/auth/logout` | Invalidate session |
| POST | `/api/auth/forgot-password` | Request reset email |
| POST | `/api/auth/reset-password` | Reset with token |
| GET | `/api/user` | Get current user |
| PUT | `/api/user/profile` | Update name/email |
| PUT | `/api/user/password` | Change password |
| DELETE | `/api/user` | Delete account |
| GET | `/health` | Health check |

## Using with Starter Kits

1. Start this backend server:
   ```bash
   cargo run
   ```

2. Start your preferred frontend:
   ```bash
   cd ../rustforge-react && npm run dev   # Port 3001
   cd ../rustforge-vue && npm run dev     # Port 3002
   cd ../rustforge-angular && npm start   # Port 3003
   ```

3. The frontend will proxy `/api` requests to this backend.

## Production Use

This is a demo implementation. For production:

1. Use proper password hashing (argon2, bcrypt)
2. Store users in a real database
3. Implement proper JWT/token management with rf-sanctum
4. Add rate limiting
5. Configure CORS properly
6. Add input validation with rf-validation

## License

MIT
