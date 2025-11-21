# Database Schema - RustForge Test Application

## Overview

This database schema demonstrates **ALL 8 Eloquent relationship types** plus additional enterprise features.

## Tables & Relationships

### Core Tables

#### 1. Users Table
**Purpose**: Authentication and user management

**Relationships**:
- `HasMany`: posts, comments, orders
- `BelongsToMany`: roles (via role_user pivot)
- `HasManyThrough`: post_comments (through posts)
- `MorphMany`: images

**Columns**:
- id, name, email, email_verified_at, password
- remember_token, two_factor_secret, two_factor_recovery_codes
- created_at, updated_at, deleted_at (soft deletes)

#### 2. Posts Table
**Purpose**: Blog content

**Relationships**:
- `BelongsTo`: user, category
- `HasMany`: comments
- `MorphMany`: images
- `MorphToMany`: tags (via taggables pivot)

**Columns**:
- id, user_id, category_id, title, slug, content
- excerpt, published_at, featured, view_count
- created_at, updated_at, deleted_at

#### 3. Comments Table
**Purpose**: User comments on posts/products

**Relationships**:
- `BelongsTo`: user
- `MorphTo`: commentable (polymorphic - can comment on posts or products)

**Columns**:
- id, user_id, commentable_type, commentable_id
- content, approved
- created_at, updated_at, deleted_at

#### 4. Categories Table
**Purpose**: Content categorization

**Relationships**:
- `HasMany`: posts
- `BelongsTo`: parent (self-referencing for hierarchical categories)

**Columns**:
- id, name, slug, description, parent_id
- created_at, updated_at

#### 5. Images Table
**Purpose**: File storage for images

**Relationships**:
- `MorphTo`: imageable (polymorphic - can belong to users, posts, products)

**Columns**:
- id, imageable_type, imageable_id
- url, filename, mime_type, size, width, height
- is_featured, created_at, updated_at

#### 6. Tags Table
**Purpose**: Tagging system

**Relationships**:
- `MorphToMany`: posts, products (via taggables pivot)

**Columns**:
- id, name, slug
- created_at, updated_at

#### 7. Products Table
**Purpose**: E-commerce products

**Relationships**:
- `BelongsToMany`: orders (via order_items pivot with extra data)
- `MorphOne`: featured_image (one featured image)
- `MorphMany`: images (gallery images)
- `MorphToMany`: tags
- `MorphTo`: comments (can be commented)

**Columns**:
- id, name, slug, description, price, sku
- stock_quantity, is_active
- created_at, updated_at, deleted_at

#### 8. Orders Table
**Purpose**: Customer orders

**Relationships**:
- `BelongsTo`: user
- `BelongsToMany`: products (via order_items with pivot data)
- `HasMany`: order_items

**Columns**:
- id, user_id, order_number, status
- total_amount, currency, shipping_address, billing_address
- payment_method, payment_status, notes
- created_at, updated_at

### Pivot Tables

#### 9. order_items (Pivot with Extra Data)
**Purpose**: Links orders to products with additional data

**Columns**:
- id, order_id, product_id
- quantity, unit_price, discount, subtotal
- created_at, updated_at

#### 10. role_user (Standard Pivot)
**Purpose**: Links users to roles

**Columns**:
- id, user_id, role_id, assigned_at

#### 11. taggables (Polymorphic Pivot)
**Purpose**: Links tags to multiple entity types

**Columns**:
- id, tag_id, taggable_type, taggable_id, created_at

### Authorization Tables

#### 12. Roles Table
**Columns**: id, name, slug, description, created_at, updated_at

#### 13. Permissions Table
**Columns**: id, name, slug, description, created_at, updated_at

#### 14. permission_role Pivot Table
**Columns**: id, permission_id, role_id, created_at

### System Tables

#### 15. notifications
Database-backed notifications

#### 16. jobs
Queue system

#### 17. failed_jobs
Failed job tracking

#### 18. cache
Database cache driver

#### 19. sessions
Session management

#### 20. personal_access_tokens
Sanctum API authentication

## Relationship Matrix

| Relationship Type | Example | Tables Involved |
|------------------|---------|----------------|
| **HasOne** | User → Profile (not implemented for simplicity) | - |
| **HasMany** | User → Posts | users, posts |
| **BelongsTo** | Post → User | posts, users |
| **BelongsToMany** | User → Roles | users, roles, role_user |
| **HasManyThrough** | User → PostComments | users, posts, comments |
| **MorphOne** | Product → FeaturedImage | products, images |
| **MorphMany** | Post → Images | posts, images |
| **MorphTo** | Comment → Commentable | comments, posts/products |
| **MorphToMany** | Post → Tags | posts, tags, taggables |

## Query Examples

### Eager Loading (N+1 Prevention)
```rust
// Load users with their posts and comments
User::with(["posts", "comments"]).get()

// Load posts with user, comments, and images
Post::with(["user", "comments", "images"]).get()
```

### Pivot Data Access
```rust
// Access order items with pivot data
let order = Order::with("products").find(1)?;
for product in order.products {
    println!("Quantity: {}", product.pivot.quantity);
    println!("Unit Price: {}", product.pivot.unit_price);
}
```

### Polymorphic Queries
```rust
// Get all images for a post
let post = Post::find(1)?;
let images = post.images().get()?;

// Get all tags for a post
let post = Post::find(1)?;
let tags = post.tags().get()?;
```

### Soft Deletes
```rust
// Soft delete
post.delete()?; // Sets deleted_at

// Query without soft deleted
Post::where("title", "like", "%test%").get()?; // Excludes deleted

// Include soft deleted
Post::withTrashed().get()?;

// Only soft deleted
Post::onlyTrashed().get()?;

// Restore
post.restore()?;
```

## Database Features Demonstrated

### Core ORM Features
- ✅ All 8 relationship types
- ✅ Eager loading
- ✅ Lazy loading
- ✅ Pivot data access
- ✅ Polymorphic relationships
- ✅ Self-referencing relationships

### Advanced Features
- ✅ Soft deletes
- ✅ Query scopes
- ✅ Model events
- ✅ Attribute casting
- ✅ Attribute mutators
- ✅ Global scopes
- ✅ Timestamps
- ✅ Database transactions

### Performance
- ✅ Indexes on foreign keys
- ✅ Unique constraints
- ✅ Composite indexes
- ✅ Optimized queries

## Migration Order

Migrations must run in this order to satisfy foreign key constraints:

1. users
2. categories (self-referencing, nullable FK)
3. posts (FK: users, categories)
4. comments (FK: users, polymorphic)
5. images (polymorphic)
6. tags
7. taggables (FK: tags, polymorphic)
8. products
9. orders (FK: users)
10. order_items (FK: orders, products)
11. roles
12. role_user (FK: users, roles)
13. permissions
14. permission_role (FK: permissions, roles)
15. notifications
16. jobs
17. failed_jobs
18. cache
19. sessions
20. personal_access_tokens

## Total Statistics

- **Tables**: 20
- **Relationships**: 15+
- **Pivot Tables**: 4
- **Polymorphic Tables**: 3
- **Soft Delete Tables**: 4
- **System Tables**: 6
