# Code Style Guidelines

## Rust Naming Conventions

### Structs and Enums
- **PascalCase** for types: `User`, `TimeEntry`, `ProjectWithStats`
- Enum variants also PascalCase: `TicketType::Task`, `AbsenceType::PaidLeave`

### Functions and Variables
- **snake_case** for functions: `create_time_entry()`, `find_by_id()`
- **snake_case** for variables: `user_id`, `start_date`

### Constants
- **SCREAMING_SNAKE_CASE**: `SESSION_COOKIE_NAME`, `PUBLIC_PATHS`

### Modules
- **snake_case** for module names: `time_entry.rs`, `azure_devops.rs`

## File Organization

### Models (`src/models/`)
- One model per file (singular form): `user.rs`, `project.rs`, `ticket.rs`
- Export all models from `mod.rs`
- Group related structs in same file (e.g., `User`, `NewUser`, `Session` in `user.rs`)

### Handlers (`src/handlers/`)
- One handler file per resource/feature
- Name handlers after action: `create_project`, `update_time_entry`, `delete_absence`
- Keep handler functions focused - delegate business logic to models or services

### Templates (`templates/`)
- Page templates: `{entity}/{action}.html` (e.g., `projects/edit.html`)
- Partials: `partials/{name}.html`
- Macros: `macros/{name}.html`

## Function Signatures

### Handler Functions
```rust
pub async fn handler_name(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    form: web::Form<FormData>,  // Optional
    path: web::Path<(i32,)>,    // Optional
) -> Result<HttpResponse>
```

### Model Methods
```rust
// Query methods
pub fn find(id: i32, conn: &mut SqliteConnection) -> QueryResult<Self>
pub fn all(conn: &mut SqliteConnection) -> QueryResult<Vec<Self>>

// Mutation methods
pub fn create(new: &NewModel, conn: &mut SqliteConnection) -> QueryResult<Self>
pub fn update(&self, conn: &mut SqliteConnection) -> QueryResult<Self>
pub fn delete(id: i32, conn: &mut SqliteConnection) -> QueryResult<usize>
```

## Diesel Patterns

### Model Traits
```rust
#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = table_name)]
pub struct Model { ... }

#[derive(Insertable)]
#[diesel(table_name = table_name)]
pub struct NewModel { ... }
```

### Query Building
- Use Diesel's query builder, avoid raw SQL
- Parameterized queries only (SQL injection prevention)
- Connection type: `&mut SqliteConnection`

## Template Patterns

### HTMX Integration
- Use `hx-get`, `hx-post` for dynamic updates
- Target specific elements with `hx-target`
- Swap content with `hx-swap="innerHTML"` or `hx-swap="outerHTML"`

### Form Handling
- Always include CSRF token: `<input type="hidden" name="csrf_token" value="{{ csrf_token }}">`
- Use semantic form elements
- Validate inputs server-side

### Partials
- Return partial HTML for HTMX requests
- Full page for regular requests
- Check `HX-Request` header to determine response type

## Error Handling

### Handler Errors
- Return `Result<HttpResponse>` or `Result<impl Responder>`
- Use generic error messages (don't leak internal details)
- Log detailed errors with tracing

### Database Errors
- Propagate `QueryResult<T>` from Diesel
- Handle `NotFound` distinctly from other errors

## Async Patterns
- Use `async fn` for all handlers
- Use `web::block` for blocking database operations
- Avoid blocking the async runtime

## Comments
- Document public APIs with `///` doc comments
- Use `//` for inline implementation notes
- Keep comments concise and relevant

## Imports
- Group imports: std, external crates, local modules
- Use explicit imports over glob imports
- Re-export commonly used items from `mod.rs`
