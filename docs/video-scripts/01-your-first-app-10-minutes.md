# Video Script: Your First RustForge App in 10 Minutes

**Target Duration:** 10 minutes
**Difficulty:** Beginner
**Target Audience:** Developers new to RustForge, familiar with web frameworks

---

## Pre-Production Notes

**Visual Style:**
- Screen recording with code editor (VS Code/IntelliJ)
- Terminal in bottom half
- Clean, professional aesthetic
- Show finished app at the start (hook)

**Required Setup:**
- Rust 1.75+ installed
- PostgreSQL running (Docker)
- Terminal ready
- Editor with Rust extensions

---

## Script

### INTRO (0:00-0:30) - 30 seconds

**[Screen: Finished app running in browser]**

**Narration:**
"In the next 10 minutes, I'm going to show you how to build a fully functional web application with RustForge. We'll create a task manager with database persistence, user authentication, and a beautiful UI. And the best part? It's as easy as Laravel, but with Rust's performance and safety."

**[Quick montage: Code editor, terminal commands, browser showing app]**

**Narration:**
"Let's get started!"

**[Transition to terminal]**

**Key Points to Emphasize:**
- 10-minute timeframe (creates urgency)
- Fully functional (not just "Hello World")
- Easy as Laravel (familiar reference point)
- Rust's benefits (performance + safety)

---

### PART 1: Installation & Setup (0:30-2:00) - 1 minute 30 seconds

**[Screen: Terminal, clean desktop]**

**Narration:**
"First, we need to install RustForge. If you don't have Rust installed yet, head to rustup.rs and install it—it takes about 2 minutes."

**[Screen: Terminal command appears]**

**Action:**
```bash
cargo install rustforge-cli
```

**Narration:**
"Run `cargo install rustforge-cli` to install the RustForge command-line tool. This will take a minute or two."

**[Time-lapse of installation, sped up 4x]**

**Narration (voiceover during time-lapse):**
"While that's installing, let me tell you what makes RustForge special. It brings Laravel's elegant API to Rust, so you get the developer experience you love, with the performance and safety of Rust. You can build web apps 10 to 100 times faster than PHP, with compile-time guarantees that catch bugs before they reach production."

**[Installation completes]**

**Action:**
```bash
forge --version
```

**Screen shows:**
```
RustForge CLI v0.1.0
```

**Narration:**
"Perfect! We're ready to build."

**Key Points:**
- Show actual installation (builds trust)
- Use time-lapse to keep pace
- Fill dead air with value proposition
- Verify installation works

---

### PART 2: Create Project (2:00-3:30) - 1 minute 30 seconds

**[Screen: Terminal]**

**Narration:**
"Let's create our project. I'll call it 'task-app'."

**Action:**
```bash
forge new task-app
cd task-app
```

**[Screen: Shows project structure being created]**

**Narration:**
"The `forge new` command scaffolds a complete application. You get routing, database setup, authentication, everything you need out of the box."

**[Show quick tree view of created files]**

**Screen shows:**
```
task-app/
├── src/
│   ├── main.rs
│   ├── routes.rs
│   ├── controllers/
│   ├── models/
│   └── views/
├── migrations/
├── .env.example
└── Cargo.toml
```

**Narration:**
"If you've used Laravel, this structure will feel familiar. Controllers handle requests, models represent data, views are your templates."

**[Open .env file]**

**Action:**
```bash
cp .env.example .env
```

**Narration:**
"Let's copy the environment file and set up our database."

**[Edit DATABASE_URL in .env]**

**Action (shown on screen):**
```env
DATABASE_URL=postgres://postgres:secret@localhost/task_app
```

**Narration:**
"I'm using PostgreSQL in Docker, but MySQL, SQLite, they all work."

**Key Points:**
- Show actual file creation
- Highlight familiar structure
- Quick environment setup
- Multiple database options mentioned

---

### PART 3: Database & Models (3:30-5:30) - 2 minutes

**[Screen: Terminal]**

**Narration:**
"Now let's create our database table for tasks."

**Action:**
```bash
forge make:migration create_tasks_table
```

**[Open created migration file]**

**Screen shows:**
`migrations/2024_01_15_create_tasks_table.rs`

**Narration:**
"This generates a migration file. Let's define our tasks table."

**[Edit migration, type out code]**

**Action (type this out live, sped up 1.5x):**
```rust
manager.create_table(
    Table::create()
        .table(Task::Table)
        .col(ColumnDef::new(Task::Id).integer().primary_key().auto_increment())
        .col(ColumnDef::new(Task::Title).string().not_null())
        .col(ColumnDef::new(Task::Completed).boolean().default(false))
        .col(ColumnDef::new(Task::CreatedAt).timestamp())
        .to_owned()
).await
```

**Narration:**
"We're defining a tasks table with an ID, title, completed status, and timestamp. The syntax is fluent and type-safe."

**[Save file, return to terminal]**

**Action:**
```bash
forge migrate
```

**Screen shows:**
```
Running migrations...
✓ create_tasks_table
Migrated successfully!
```

**Narration:**
"Run `forge migrate` and our table is created."

**[Screen: Create model file]**

**Action:**
```bash
forge make:model Task
```

**[Open created model file src/models/task.rs]**

**Action (show completed model):**
```rust
#[derive(Model, Serialize, Deserialize)]
pub struct Task {
    pub id: i32,
    pub title: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
}
```

**Narration:**
"Our Task model. Simple, clean, and fully typed. The compiler ensures we never forget a field."

**Key Points:**
- Show real migration creation
- Type code to show syntax
- Emphasize type safety
- Quick, smooth workflow

---

### PART 4: Routes & Controllers (5:30-7:30) - 2 minutes

**[Screen: Editor, open src/routes.rs]**

**Narration:**
"Let's add routes for our tasks."

**Action (type out, sped up 1.5x):**
```rust
Router::new()
    .route("/", Route::get(task_controller::index))
    .route("/tasks", Route::post(task_controller::store))
    .route("/tasks/:id/complete", Route::post(task_controller::complete))
    .route("/tasks/:id", Route::delete(task_controller::destroy))
```

**Narration:**
"Four routes: list tasks, create a task, mark complete, and delete. RESTful and clean."

**[Screen: Create controller]**

**Action:**
```bash
forge make:controller TaskController
```

**[Open src/controllers/task_controller.rs]**

**Narration:**
"Now the controller. Let's build the index action that lists all tasks."

**Action (type out):**
```rust
pub async fn index(req: Request) -> Response {
    let tasks = Task::all(req.db()).await.unwrap();
    View::make("tasks.index").with("tasks", tasks).render()
}
```

**Narration:**
"Three lines. Fetch all tasks from the database, pass them to the view, render. Beautiful."

**[Split screen: Show store action being typed on left, completed code on right]**

**Action (type on left):**
```rust
pub async fn store(req: Request) -> Response {
    let title = req.input::<String>("title").unwrap();

    Task::create(req.db(), TaskData {
        title,
        completed: false,
    }).await.unwrap();

    Response::redirect("/")
}
```

**Narration:**
"The store action creates a new task. Get the title from the request, create the task, redirect back. Simple as that."

**[Show complete and destroy actions quickly, don't type them out]**

**Narration:**
"I'll skip typing the complete and destroy actions—they follow the same pattern. The code is in the description."

**Key Points:**
- Show routing syntax
- Type code for realism
- Highlight simplicity (3 lines)
- Mention full code in description

---

### PART 5: Views (7:30-9:00) - 1 minute 30 seconds

**[Screen: Create view file src/views/tasks/index.blade.html]**

**Narration:**
"Now for the view. RustForge uses Blade templates, so if you know Laravel, you already know this."

**Action (show completed template, scroll through):**
```html
<!DOCTYPE html>
<html>
<head>
    <title>Task App</title>
    <style>
        body { font-family: sans-serif; max-width: 600px; margin: 50px auto; }
        .task { padding: 10px; border: 1px solid #ddd; margin: 10px 0; }
        .completed { text-decoration: line-through; color: #999; }
    </style>
</head>
<body>
    <h1>My Tasks</h1>

    <form method="POST" action="/tasks">
        @csrf
        <input type="text" name="title" placeholder="New task..." />
        <button>Add</button>
    </form>

    @foreach(task in tasks)
        <div class="task {{ task.completed ? 'completed' : '' }}">
            <span>{{ task.title }}</span>

            @if(!task.completed)
                <form method="POST" action="/tasks/{{ task.id }}/complete" style="display:inline">
                    @csrf
                    <button>✓</button>
                </form>
            @endif

            <form method="POST" action="/tasks/{{ task.id }}" style="display:inline">
                @csrf
                @method('DELETE')
                <button>✗</button>
            </form>
        </div>
    @endforeach
</body>
</html>
```

**Narration:**
"We've got a form to create tasks, and we loop through existing tasks with `@foreach`. Each task shows a complete button and a delete button. Simple HTML with Blade directives."

**Key Points:**
- Show finished template (don't type it all)
- Highlight Blade syntax
- Simple, functional UI
- Laravel developers recognize it

---

### PART 6: Run & Demo (9:00-10:00) - 1 minute

**[Screen: Terminal]**

**Narration:**
"Moment of truth. Let's run our app."

**Action:**
```bash
forge serve
```

**Screen shows:**
```
🚀 RustForge server starting...
📍 Listening on http://127.0.0.1:8000
✅ Server ready!
```

**Narration:**
"In under 10 seconds, we're live."

**[Screen: Browser opens to localhost:8000]**

**Narration:**
"And there it is. Let's add some tasks."

**[Demo: Add 3 tasks, complete one, delete one]**

**Actions shown:**
1. Type "Buy milk" → Add → appears in list
2. Type "Walk dog" → Add → appears in list
3. Type "Learn Rust" → Add → appears in list
4. Click complete on "Walk dog" → grays out
5. Click delete on "Buy milk" → disappears

**Narration:**
"Add a task, complete a task, delete a task. Full CRUD in 10 minutes. And this isn't just a toy app—this is production-ready code. It's using PostgreSQL, it's fully typed, it's safe, it's fast."

**[Screen: Split view—app on left, terminal with stats on right]**

**Screen shows:**
```
Response time: 2ms
Memory usage: 12MB
```

**Narration:**
"Two millisecond response time, 12 megabytes of memory. Try doing that with PHP."

**Key Points:**
- Fast startup (builds excitement)
- Real interaction (not static)
- Emphasize production-ready
- Show performance stats (wow factor)

---

### OUTRO (10:00-10:30) - 30 seconds

**[Screen: Recap—split screen of code, app, terminal]**

**Narration:**
"In 10 minutes, we installed RustForge, created a project, set up a database, built models, controllers, and views, and deployed a working task manager. This is the power of RustForge: Laravel's simplicity meets Rust's performance."

**[Screen: Call to action overlay]**

**Text on screen:**
```
🚀 Try RustForge Today
📚 docs.rustforge.dev
💬 discord.gg/rustforge
⭐ github.com/rustforge/rustforge
```

**Narration:**
"If you want to learn more, check out the docs at rustforge.dev. Join our Discord to chat with the community. And if you enjoyed this video, give it a like and subscribe for more Rust tutorials. Thanks for watching, and happy coding!"

**[Music swells, fade to black]**

**Key Points:**
- Recap accomplishments
- Emphasize speed (10 minutes)
- Clear call to action
- Multiple engagement options

---

## Post-Production Checklist

### Editing
- [ ] Add timestamps in description
- [ ] Highlight key commands (text overlay)
- [ ] Add background music (low volume, non-distracting)
- [ ] Speed up installation/compilation sequences
- [ ] Add "Subscribe" animation at 9:45
- [ ] Color-grade for consistency

### Captions
- [ ] Generate accurate captions
- [ ] Review and fix auto-caption errors
- [ ] Time captions to match speech

### Thumbnail
- [ ] "10 Minutes" prominent
- [ ] "Your First App" as subtitle
- [ ] RustForge logo
- [ ] Contrasting colors (high CTR)

### Description
```markdown
Build your first RustForge web app in just 10 minutes! We'll create a complete task manager with database, CRUD operations, and a beautiful UI.

⏱️ Timestamps:
0:00 - Intro
0:30 - Installation
2:00 - Create Project
3:30 - Database & Models
5:30 - Routes & Controllers
7:30 - Views
9:00 - Run & Demo
10:00 - Outro

📚 Resources:
- Full code: github.com/rustforge/examples/task-app
- Docs: docs.rustforge.dev
- Discord: discord.gg/rustforge

#rust #rustlang #rustforge #webdev #tutorial
```

---

## Script Notes for Presenter

### Tone & Delivery
- **Energetic but not frantic** - Keep pace up, but clear
- **Confident** - You know what you're doing
- **Friendly** - Approachable, not condescending
- **Enthusiastic** - Genuinely excited about the framework

### Common Mistakes to Avoid
- ❌ Don't say "um," "uh," "like" - edit them out
- ❌ Don't apologize for mistakes - restart the segment
- ❌ Don't rush through code - viewers need to process
- ❌ Don't assume knowledge - explain briefly

### Tips for Recording
- ✅ Record audio separately (better quality control)
- ✅ Use a pop filter (reduce plosives)
- ✅ Record in 4K (future-proof, better zoom crops)
- ✅ Use code snippets (don't type everything live)
- ✅ Record multiple takes (pick the best)

### What to Show vs. What to Skip
- **Show:** Installation, migration, routing, one controller
- **Skip:** Typing repetitive code (copy-paste is fine)
- **Time-lapse:** Compilation, installation
- **Quick show:** Finished views (don't type HTML)

---

## Technical Setup

### Screen Recording Settings
- **Resolution:** 1920x1080 (minimum)
- **Frame rate:** 60fps (for smooth terminal/browser)
- **Bitrate:** High (crisp code text)
- **Cursor:** Visible and large
- **Audio:** 48kHz, stereo

### Code Editor Setup
- **Theme:** High contrast (e.g., Dracula, One Dark Pro)
- **Font size:** 16-18pt (readable at 1080p)
- **Hide distractions:** Minimap, breadcrumbs off
- **Terminal:** Bottom panel, 40% height

### Browser Setup
- **Clean profile:** No extensions visible
- **Window size:** Fixed 1200x800
- **Zoom:** 125% (larger, more readable)

---

## Alternative Versions

### 5-Minute Speed Run
- Skip installation (assume installed)
- Pre-created migration
- Show finished code, explain quickly
- Focus on "wow" factor

### 20-Minute Deep Dive
- Explain every line
- Add validation
- Add authentication
- Deploy to production
- Show testing

### Series Format
- **Video 1:** This script (overview)
- **Video 2:** Deep dive into routing
- **Video 3:** Database relationships
- **Video 4:** Authentication
- **Video 5:** Deployment

---

This script is designed to be:
- ✅ **Achievable** in 10 minutes
- ✅ **Impressive** (full CRUD app)
- ✅ **Professional** (production code)
- ✅ **Engaging** (fast-paced, visual)
- ✅ **Valuable** (viewers learn and get excited)

**Ready to record!** 🎥
