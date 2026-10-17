# 🚀 Rocket + Diesel Clean Architecture

A modular, scalable, and easy-to-reason-about Rust backend boilerplate using **Rocket 0.5**, **Diesel 2.2**, and **PostgreSQL (Neon DB)**.

---

## 🧠 The Architecture in Plain English

Most backend codebases become a chaotic mess because developers mix database queries, validation, business logic, and HTTP endpoints in the same files.

This project enforces a strict 4-layer flow:

```
Request ➡️ [ Controllers ] ➡️ [ Services ] ➡️ [ Usecases ] ➡️ [ Database ]
Response ⬅️ [ Controllers ] ⬅️ [ Services ] ⬅️ [ Usecases ] ⬅️ [ Database ]
```

---

## 🏛️ The 4 Layers Explained

```mermaid
graph TD
    Client["Client / HTTP Request"] --> Controller["1. Controller & Interface<br/>(HTTP routes & DTOs)"]
    Controller --> Service["2. Service<br/>(Library of Usecases)"]
    Service --> Usecase["3. Usecase (1 File per Action)<br/>(create_user.rs, get_user.rs)"]
    Usecase --> DB["4. Database<br/>(Models, Schema, Connection)"]
```

### 1. Database (`src/database/`) — *Where data definitions live*
- **Purpose**: Defines database tables and establishes connections.
- **Rules**:
  - Contains **only** entity structs (`models.rs`), table definitions (`schema.rs`), and connection logic (`connection.rs`).
  - **No business logic or CRUD functions go here**.

### 2. Usecases (`src/usecases/`) — *One action, one file*
- **Purpose**: Does the actual work and runs the raw Diesel CRUD queries.
- **Rules**:
  - **1 file = 1 single business operation** (e.g., `create_user.rs`, `get_user.rs`).
  - Foldered strictly by table/entity (e.g., `src/usecases/user/`).
  - Each file exposes a single `pub fn execute(...)` function.

### 3. Services (`src/services/`) — *The shelf/library of usecases*
- **Purpose**: Acts as the clean public API for a domain.
- **Rules**:
  - Grouped per entity (e.g., `user_service.rs` for users only).
  - **Boundary rule**: `UserService` can **only** call usecases from `usecases/user/`.
  - Controllers never call usecases directly; they always ask the Service.

### 4. Controllers & Interfaces (`src/controllers/`) — *The front door*
- **Purpose**: Listens for HTTP requests (GET, POST, DELETE) and returns JSON responses.
- **Rules**:
  - Foldered per domain entity (e.g. `src/controllers/products/`):
    - `<singular>.interface.rs`: Defines the controller contract as a trait (`IProductControllerInterface`) and DTO structs (`CreateProductPayload`, `ProductResponse<T>`).
    - `<singular>.controller.rs`: Implements the interface trait on a controller struct (`ProductController`), and defines Rocket route handlers that delegate to it.
  - A controller can call **multiple services** (e.g., `UserService` + `NotificationService`), but **never calls usecases directly**.

---

## 📁 Project Folder Layout

```text
src/
├── database/                    <-- Where data shapes and connections live
│   ├── mod.rs
│   ├── connection.rs            <-- Postgres connection (reads .env DATABASE_URL)
│   ├── schema.rs                <-- Diesel table! macro
│   └── models.rs                <-- Structs (Queryable, Selectable, Insertable)
│
├── usecases/                    <-- 1 action per file, foldered by table
│   ├── mod.rs
│   ├── user/
│   │   ├── mod.rs
│   │   ├── create_user.rs       <-- Inserts user record via Diesel
│   │   ├── get_user.rs          <-- Fetches single user by ID
│   │   ├── get_all_users.rs     <-- Queries all users
│   │   └── delete_user.rs       <-- Deletes user by ID
│   └── product/
│       ├── mod.rs
│       ├── create_product.rs
│       ├── get_product.rs
│       ├── get_all_products.rs
│       └── delete_product.rs
│
├── services/                    <-- Facades for usecases (per-domain only)
│   ├── mod.rs
│   ├── user_service.rs          <-- Calls usecases::user::*
│   └── product_service.rs       <-- Calls usecases::product::*
│
├── controllers/                 <-- Route handlers & interfaces foldered per domain
│   ├── mod.rs
│   ├── users/
│   │   ├── mod.rs
│   │   ├── user.interface.rs    <-- Controller Trait contract & DTOs
│   │   └── user.controller.rs   <-- UserController struct (impl trait) & Rocket routes
│   └── products/
│       ├── mod.rs
│       ├── product.interface.rs <-- Controller Trait contract & DTOs
│       └── product.controller.rs<-- ProductController struct (impl trait) & Rocket routes
│
└── main.rs                      <-- Mounts controller routes to Rocket
```

---

## ⚡ Quickstart Guide

### 1. Prerequisites
- Rust (latest stable): `rustup default stable`
- PostgreSQL connection URL (e.g. [Neon](https://neon.tech))

### 2. Environment Setup
Create a `.env` file in the project root:

```env
DATABASE_URL=postgresql://username:password@ep-host.ap-southeast-1.aws.neon.tech/neondb?sslmode=require
```

### 3. Run Migrations
Run pending database migrations against Neon:

```powershell
diesel migration run
```

### 4. Start the Server
```powershell
cargo run
```
The server will boot at: **`http://127.0.0.1:8000`**

---

## 🪄 Generating New Tables & CRUD with Cargo

Instead of writing all 4 layers manually for every new table, use the built-in Cargo generator command:

```bash
cargo gen <table_name>
```

*(Works cross-platform on Windows, Linux, and macOS without extra scripting tools)*.

### Example:
```bash
# Creates & migrates 'categories' table on Neon and scaffolds all 4 layers:
cargo gen categories

# Or run interactively (will prompt for table name):
cargo gen
```

> **Note**: `cargo gen` is an alias configured in `.cargo/config.toml` that runs `cargo run --bin gen -- <table_name>`.

### What the script does automatically:
1. **Creates & Runs Neon Migration** with default audit fields:
   - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
   - `name VARCHAR NOT NULL`
   - `created_at TIMESTAMPTZ DEFAULT NOW()`
   - `updated_at TIMESTAMPTZ DEFAULT NOW()`
   - `created_by VARCHAR DEFAULT 'system'`
   - `updated_by VARCHAR DEFAULT 'system'`
2. **Generates Model** in `src/database/models.rs`
3. **Generates 4 Usecases** under `src/usecases/<singular>/`
4. **Generates Service** in `src/services/<singular>_service.rs`
5. **Generates Interface** in `src/controllers/<plural>/<singular>.interface.rs` with `I<Singular>ControllerInterface` trait contract and DTOs
6. **Generates Controller** in `src/controllers/<plural>/<singular>.controller.rs` with `<Singular>Controller` struct implementing the interface, and Rocket route handlers
7. **Mounts the route** in `src/main.rs` (`.mount("/<plural>", controllers::<plural>::controller::get_routes())`)
8. **Runs `cargo check`** to verify everything compiles cleanly!

---

## ⚡ Generating Individual API Routes with Cargo

To scaffold an individual custom API route/endpoint into an existing or new domain without generating database tables:

```bash
cargo gen-api
```

Or pass arguments directly:
```bash
cargo gen-api users change_password POST /<id>/change-password
cargo gen-api products search GET /search
```

### Interactive Questions:
1. **Target domain** (`users`, `products`, etc.)
2. **Action name** (`change_password`, `search`, etc.)
3. **HTTP Method** (`GET`, `POST`, `PUT`, `DELETE`, `PATCH`)
4. **Endpoint Subpath** (`/<id>/change-password`, `/search`)
5. **Authentication?** `[Y/n]` (default: `Y` - protected by default)
6. **If Public (`n`)**:
   - `Which method(s) to open in PublicRoutes?`
     - `1: <METHOD> only` (e.g. `GET /products/search` public, other methods protected)
     - `2: ALL methods for this path`

The script automatically wires the **Usecase**, **Service**, **Interface trait**, **Controller handler & routes!**, and registers public routes in `src/main.rs`.

---

## ✍️ How to Add a Route Manually (Step-by-Step)

If you ever want to add an endpoint manually (e.g., `POST /users/<id>/change-password`):

1. **Usecase** (`src/usecases/user/change_password.rs`):
   - Write the Diesel query in a standalone `execute(user_id, new_password)` function.
   - Register in `src/usecases/user/mod.rs`.
2. **Service** (`src/services/user_service.rs`):
   - Add a forwarding method `change_password` that calls `change_password::execute(...)`.
3. **Interface** (`src/controllers/users/user.interface.rs`):
   - Add the request payload DTO:
     ```rust
     #[derive(Debug, Deserialize, Serialize)]
     pub struct ChangePasswordPayload {
         pub new_password: String,
     }
     ```
   - Add the method signature to the controller trait contract:
     ```rust
     pub trait IUserControllerInterface {
         // ... existing methods ...
         fn change_password(id: i32, payload: Json<ChangePasswordPayload>) -> UserResponse<String>;
     }
     ```
4. **Controller** (`src/controllers/users/user.controller.rs`):
   - Implement the method inside `impl IUserControllerInterface for UserController`:
     ```rust
     fn change_password(id: i32, payload: Json<ChangePasswordPayload>) -> UserResponse<String> {
         UserService::change_password(id, &payload.new_password)
             .map(|msg| Json(msg))
             .map_err(|err| error_response(Status::BadRequest, err))
     }
     ```
   - Define the Rocket HTTP route handler that delegates to `UserController`:
     ```rust
     #[post("/<id>/change-password", data = "<payload>")]
     fn change_password(id: i32, payload: Json<ChangePasswordPayload>) -> UserResponse<String> {
         UserController::change_password(id, payload)
     }
     ```
   - Add the function name to `routes![...]` at the bottom of the controller.
5. **Test**: Run `cargo check` and `cargo run`.

---

## 🛡️ The Golden Rules to Remember

| Do | Don't |
|---|---|
| ✅ Keep each usecase in its own individual `.rs` file. | ❌ Don't dump multiple usecases into one giant file. |
| ✅ Call **Services** from your Controllers. | ❌ Never import or call Usecases directly inside Controllers. |
| ✅ Call raw Diesel queries inside **Usecases**. | ❌ Don't write SQL or Diesel queries inside Controllers or Services. |
| ✅ Define traits & request/response DTOs inside `<name>.interface.rs`. | ❌ Don't pollute the controller with raw structs. |
| ✅ Keep controllers in dedicated domain folders (`controllers/<domain>/`). | ❌ Don't put loose flat controller files directly in `src/controllers/`. |
