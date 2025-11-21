# Building a Blog with RustForge

**Total Time:** 4-5 hours
**Difficulty:** Beginner to Intermediate
**Prerequisites:** Complete [Getting Started Tutorial](../01-getting-started.md)

---

## What You'll Build

A fully functional blog application with:

- ✅ User registration and authentication
- ✅ Create, read, update, delete posts
- ✅ Comments system
- ✅ Markdown support for posts
- ✅ Image uploads for post covers
- ✅ Search functionality
- ✅ Pagination
- ✅ Author profiles
- ✅ Authorization (only authors can edit their posts)

---

## Tutorial Chapters

### Part 1: Foundation (1 hour)

**[Chapter 1: Project Setup](./01-setup.md)** (15 min)
- Create the project
- Configure database
- Set up environment

**[Chapter 2: Database & Migrations](./02-database.md)** (20 min)
- Create database tables
- Write migrations
- Understand relationships

**[Chapter 3: User Model & Authentication](./03-authentication.md)** (25 min)
- User model
- Registration
- Login/Logout
- Session management

### Part 2: Core Features (2 hours)

**[Chapter 4: Creating Posts](./04-posts.md)** (30 min)
- Post model
- CRUD operations
- Form validation
- Eloquent relationships

**[Chapter 5: Blade Templates](./05-templates.md)** (30 min)
- Layout system
- Component reuse
- Blade directives
- Template inheritance

**[Chapter 6: Adding Comments](./06-comments.md)** (30 min)
- Comment model
- Nested relationships
- Real-time updates

**[Chapter 7: File Uploads](./07-uploads.md)** (30 min)
- Image handling
- Storage configuration
- File validation
- Serving files

### Part 3: Advanced Features (1-1.5 hours)

**[Chapter 8: Search Functionality](./08-search.md)** (20 min)
- Full-text search
- Filter posts
- Query optimization

**[Chapter 9: Authorization](./09-authorization.md)** (25 min)
- Gates and policies
- Middleware
- Resource authorization

**[Chapter 10: Deployment](./10-deployment.md)** (25 min)
- Production configuration
- Docker setup
- Deploy to cloud
- Monitoring

---

## What You'll Learn

### Database & ORM
- Migrations and schema design
- Eloquent relationships (HasMany, BelongsTo)
- Eager loading to prevent N+1 queries
- Query builder
- Database seeding

### Authentication & Authorization
- User registration flow
- Password hashing
- Session management
- Login/logout
- Gates and policies
- Middleware

### Views & Templates
- Blade template engine
- Template inheritance
- Components
- Directives (@if, @foreach, @auth)
- CSRF protection

### Forms & Validation
- Form requests
- Validation rules
- Custom error messages
- File uploads
- Old input

### File Management
- File storage
- Image uploads
- File validation
- Public/private storage

---

## Prerequisites

Before starting, make sure you have:

1. **Completed the [Getting Started Tutorial](../01-getting-started.md)**
2. **PostgreSQL installed** (or Docker)
3. **Basic Rust knowledge** (ownership, async/await)
4. **Text editor** (VS Code, IntelliJ, etc.)

---

## Getting Help

If you get stuck:

1. Check the [Troubleshooting](#troubleshooting) section in each chapter
2. Review the [complete source code](https://github.com/rustforge/examples/tree/main/blog)
3. Ask on [Discord](https://discord.gg/rustforge)
4. Open an issue on [GitHub](https://github.com/rustforge/rustforge/issues)

---

## Final Result

By the end of this tutorial, you'll have built a blog that looks like this:

```
┌─────────────────────────────────────────┐
│  RustForge Blog                    Login│
├─────────────────────────────────────────┤
│  Search: [____________]  [Search]       │
├─────────────────────────────────────────┤
│  ┌────────────────────────────────┐    │
│  │ Getting Started with Rust       │    │
│  │ By Alice • 2 days ago           │    │
│  │ Learn the basics of Rust...     │    │
│  │ [Read More] [Edit] [Delete]     │    │
│  └────────────────────────────────┘    │
│                                         │
│  ┌────────────────────────────────┐    │
│  │ Advanced Async Patterns         │    │
│  │ By Bob • 5 days ago             │    │
│  │ Deep dive into async/await...   │    │
│  │ [Read More]                     │    │
│  └────────────────────────────────┘    │
│                                         │
│  « Previous | 1 2 3 4 | Next »         │
└─────────────────────────────────────────┘
```

---

## Let's Get Started!

Ready to build your blog? Head to **[Chapter 1: Project Setup](./01-setup.md)** to begin!

---

## Troubleshooting

### Common Issues

**Database connection failed**
- Make sure PostgreSQL is running
- Check your `.env` DATABASE_URL
- Verify credentials

**Compilation errors**
- Run `cargo clean && cargo build`
- Check Rust version: `rustc --version` (need 1.75+)

**Template not found**
- Ensure `.blade.html` extension
- Check file is in `src/views/`

**Session not persisting**
- Check APP_KEY is set in `.env`
- Clear cookies and try again

---

## Source Code

The complete source code for this tutorial is available at:

**GitHub:** [github.com/rustforge/examples/tree/main/blog](https://github.com/rustforge/examples/tree/main/blog)

You can clone it to see the final result:

```bash
git clone https://github.com/rustforge/examples.git
cd examples/blog
cargo run
```

---

Ready? Let's build something amazing! 🚀

**Next:** [Chapter 1: Project Setup](./01-setup.md)
