# Phase 15: Advanced Query Builder & Broadcasting Features

## Overview

Phase 15 implements advanced database query features and significantly enhances the broadcasting system with a JavaScript client and presence channels. This phase brings RustForge to feature parity with Laravel's most advanced database and real-time capabilities.

## Goals

1. **Advanced Query Builder**: Subqueries, unions, raw expressions, aggregates, chunking, locking
2. **Broadcasting Enhancements**: JavaScript client, private channels, presence channels
3. **Laravel Parity**: Achieve 100% parity for advanced ORM and broadcasting features
4. **Production Ready**: All features production-tested and documented

## Part 1: Advanced Query Builder Features

### 1.1 Subqueries

Support subqueries in WHERE clauses and SELECT statements:

```rust
// WHERE IN with subquery
Post::where_in("user_id",
    User::where("active", true).select("id")
).get().await?;

// Subquery in SELECT
Post::select([
    "posts.*",
    "(SELECT COUNT(*) FROM comments WHERE comments.post_id = posts.id) as comment_count"
]).get().await?;

// EXISTS subquery
Post::where_exists(
    Comment::where_column("comments.post_id", "posts.id")
).get().await?;
```

### 1.2 Unions

Combine multiple query results:

```rust
// Basic union
let published = Post::where("published", true);
let featured = Post::where("featured", true);
published.union(featured).get().await?;

// Union all (with duplicates)
Post::where("status", "draft")
    .union_all(Post::where("status", "pending"))
    .get()
    .await?;
```

### 1.3 Raw Expressions

Execute raw SQL within the query builder:

```rust
// Raw SELECT
Post::select_raw("COUNT(*) as total, DATE(created_at) as date")
    .group_by("date")
    .get()
    .await?;

// Raw WHERE
Post::where_raw("DATE(created_at) = ?", [today])
    .where_raw("views > ?", [1000])
    .get()
    .await?;

// Raw ORDER BY
Post::order_by_raw("FIELD(status, 'published', 'draft', 'archived')")
    .get()
    .await?;
```

### 1.4 Aggregates

Built-in aggregate functions:

```rust
// Count
let total = Post::count().await?;
let published = Post::where("published", true).count().await?;

// Sum
let total_views = Post::sum("views").await?;

// Average
let avg_rating = Post::avg("rating").await?;

// Min/Max
let min_price = Product::min("price").await?;
let max_price = Product::max("price").await?;

// Multiple aggregates
let stats = Post::aggregate([
    Aggregate::count("*").as_("total"),
    Aggregate::avg("rating").as_("avg_rating"),
    Aggregate::sum("views").as_("total_views"),
]).first().await?;
```

### 1.5 Chunking

Process large datasets in chunks to avoid memory issues:

```rust
// Process 100 records at a time
Post::chunk(100, |posts| async {
    for post in posts {
        process_post(post).await?;
    }
    Ok(())
}).await?;

// Chunk by ID (safer for updates)
Post::chunk_by_id(100, |posts| async {
    for post in posts {
        update_post(post).await?;
    }
    Ok(())
}, "id").await?;

// Lazy iteration (memory efficient)
Post::lazy(100).for_each(|post| async {
    process_post(post).await?;
}).await?;
```

### 1.6 Pessimistic Locking

Lock rows for concurrent updates:

```rust
// Exclusive lock (FOR UPDATE)
let post = Post::lock_for_update()
    .find(1)
    .await?;

// Shared lock (FOR SHARE / LOCK IN SHARE MODE)
let post = Post::shared_lock()
    .find(1)
    .await?;

// Skip locked rows (SKIP LOCKED)
let posts = Post::lock_for_update()
    .skip_locked()
    .limit(10)
    .get()
    .await?;

// Wait for locks with timeout (NOWAIT)
let post = Post::lock_for_update()
    .no_wait()
    .find(1)
    .await?;
```

## Part 2: Event Broadcasting Improvements

### 2.1 JavaScript Client (rustforge-echo)

NPM package for WebSocket client:

```javascript
// Installation
npm install rustforge-echo

// Basic usage
import Echo from 'rustforge-echo';

const echo = new Echo({
    broadcaster: 'websocket',
    host: 'localhost:8000',
    path: '/ws',
});

// Listen to public channel
echo.channel('posts')
    .listen('PostPublished', (event) => {
        console.log('New post:', event.post);
    });

// Listen to multiple events
echo.channel('notifications')
    .listen('OrderPlaced', handleOrder)
    .listen('OrderShipped', handleShipment)
    .listen('OrderDelivered', handleDelivery);
```

### 2.2 Private Channels with Auth

Authenticated, user-specific channels:

```rust
// Server-side (Rust)
#[derive(Broadcast)]
#[channel("user.{user_id}")]
struct MessageReceived {
    user_id: i32,
    message: Message,
}

// Authorize channel access
impl BroadcastAuth for MessageReceived {
    fn authorize(&self, user: &User) -> bool {
        user.id == self.user_id
    }
}

// Broadcast
MessageReceived {
    user_id: 1,
    message,
}.broadcast().await?;
```

```javascript
// Client-side (JavaScript)
echo.private('user.1')
    .listen('MessageReceived', (event) => {
        console.log('New message:', event.message);
    });

// Authentication endpoint
echo.connector.authorizer = (channel) => {
    return fetch('/broadcasting/auth', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${token}`,
        },
        body: JSON.stringify({ channel_name: channel.name }),
    });
};
```

### 2.3 Presence Channels

Track who's online/active on a channel:

```rust
// Server-side
#[derive(Broadcast)]
#[channel("chat")]
#[presence]
struct UserJoined {
    user: User,
}

impl PresenceChannel for UserJoined {
    fn presence_data(&self) -> Value {
        json!({
            "id": self.user.id,
            "name": self.user.name,
            "avatar": self.user.avatar_url,
        })
    }
}
```

```javascript
// Client-side
echo.join('chat')
    .here((users) => {
        // Users currently in the channel
        console.log('Users here:', users);
        updateUserList(users);
    })
    .joining((user) => {
        // Someone joined
        console.log('User joined:', user);
        addUserToList(user);
    })
    .leaving((user) => {
        // Someone left
        console.log('User left:', user);
        removeUserFromList(user);
    })
    .listen('MessageSent', (event) => {
        // Regular events work too
        displayMessage(event.message);
    });

// Get current members
const members = echo.join('chat').members();
console.log(`${members.length} users online`);
```

## Implementation Structure

### Query Builder Extensions (rf-orm)

```
crates/rf-orm/src/
├── query/
│   ├── builder.rs          # Extended with new methods
│   ├── subquery.rs         # NEW: Subquery support
│   ├── union.rs            # NEW: Union operations
│   ├── raw.rs              # NEW: Raw expressions
│   ├── aggregate.rs        # NEW: Aggregate functions
│   ├── chunk.rs            # NEW: Chunking iterators
│   └── lock.rs             # NEW: Pessimistic locking
└── tests/
    └── advanced_query.rs   # NEW: Comprehensive tests
```

### Broadcasting Extensions (rf-broadcast)

```
crates/rf-broadcast/src/
├── channels/
│   ├── private.rs          # NEW: Private channel auth
│   └── presence.rs         # NEW: Presence tracking
├── auth/
│   └── authorizer.rs       # NEW: Channel authorization
└── tests/
    └── presence.rs         # NEW: Presence tests
```

### JavaScript Client Package

```
packages/rustforge-echo/
├── src/
│   ├── echo.js             # Main Echo class
│   ├── channel.js          # Channel class
│   ├── private-channel.js  # Private channel
│   ├── presence-channel.js # Presence channel
│   └── connector/
│       ├── websocket.js    # WebSocket connector
│       └── socketio.js     # Socket.IO connector
├── package.json
├── README.md
└── examples/
    └── chat-app.html
```

## Testing Strategy

### Query Builder Tests
- [ ] Subquery generation SQL correctness
- [ ] Union query execution
- [ ] Raw expression safety (SQL injection prevention)
- [ ] Aggregate calculation accuracy
- [ ] Chunking memory efficiency
- [ ] Lock behavior under concurrency

### Broadcasting Tests
- [ ] Private channel authorization
- [ ] Presence join/leave events
- [ ] JavaScript client connection
- [ ] Multi-user presence scenarios
- [ ] Authentication flow
- [ ] Error handling and reconnection

## Laravel Feature Comparison

| Feature | Laravel | RustForge Phase 15 | Status |
|---------|---------|-------------------|---------|
| Subqueries | ✅ | ✅ | Implementing |
| Unions | ✅ | ✅ | Implementing |
| Raw Expressions | ✅ | ✅ | Implementing |
| Aggregates | ✅ | ✅ | Implementing |
| Chunking | ✅ | ✅ | Implementing |
| Locking | ✅ | ✅ | Implementing |
| Private Channels | ✅ | ✅ | Implementing |
| Presence Channels | ✅ | ✅ | Implementing |
| Laravel Echo | ✅ | RustForge Echo ✅ | Implementing |

## Success Criteria

- [ ] All query builder features work with SQLite, PostgreSQL, MySQL
- [ ] Chunking handles millions of records efficiently
- [ ] Locking prevents race conditions in tests
- [ ] JavaScript client connects and receives events
- [ ] Private channels reject unauthorized users
- [ ] Presence channels track 100+ concurrent users
- [ ] All features have >95% test coverage
- [ ] Documentation with examples for all features

## Dependencies

```toml
# rf-orm additions
[dependencies]
futures = "0.3"  # For async iterators (chunking)

# rf-broadcast additions
[dependencies]
tower = "0.4"    # For auth middleware
axum = "0.7"     # For auth endpoints

# rustforge-echo (JavaScript)
{
  "dependencies": {
    "ws": "^8.14.0",
    "axios": "^1.6.0"
  }
}
```

## Timeline

- Phase 15.1: Query Builder Extensions (5 days)
  - Day 1-2: Subqueries, Unions
  - Day 3: Raw Expressions
  - Day 4: Aggregates, Chunking
  - Day 5: Locking

- Phase 15.2: Broadcasting (5 days)
  - Day 1-2: JavaScript Client
  - Day 3: Private Channels
  - Day 4: Presence Channels
  - Day 5: Integration & Testing

- Testing & Documentation (2 days)

**Total: ~12 days**

## Notes

- Subqueries must properly escape parameters to prevent SQL injection
- Chunking should use cursors/keyset pagination for large datasets
- Locking behavior differs between databases (document limitations)
- Presence channels require Redis or similar for state management
- JavaScript client should support reconnection and offline queuing

## Future Enhancements (Phase 16+)

- Query caching layer
- Read replicas support
- Database sharding
- GraphQL subscriptions over WebSockets
- Voice/Video channel support in presence
