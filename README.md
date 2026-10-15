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
  - Every controller has its own interface file:
    - `<name>_interface.rs`: Defines request bodies (`CreateUserPayload`) and response types (`UserResponse<T>`).
    - `<name>_controller.rs`: Declares Rocket routes and calls the appropriate Service.
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
├── controllers/                 <-- Route handlers & interfaces
│   ├── mod.rs
│   ├── user_interface.rs        <-- User DTOs (CreateUserPayload, UserResponse)
│   ├── user_controller.rs       <-- Rocket routes for /users
│   ├── product_interface.rs     <-- Product DTOs
│   └── product_controller.rs    <-- Rocket routes for /products
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
5. **Generates Interface** in `src/controllers/<singular>_interface.rs`
6. **Generates Controller** in `src/controllers/<singular>_controller.rs`
7. **Mounts the route** in `src/main.rs`
8. **Runs `cargo check`** to verify everything compiles cleanly!

---

## ✍️ How to Add a Route Manually (Step-by-Step)

If you ever want to add an endpoint manually (e.g., `POST /users/change-password`):

1. **Usecase** (`src/usecases/user/change_password.rs`):
   - Write the Diesel query in a standalone `execute(user_id, new_password)` function.
   - Register in `src/usecases/user/mod.rs`.
2. **Service** (`src/services/user_service.rs`):
   - Add a forwarding method `change_password` that calls `change_password::execute(...)`.
3. **Interface** (`src/controllers/user_interface.rs`):
   - Add the request payload: `ChangePasswordPayload { new_password: String }`.
4. **Controller** (`src/controllers/user_controller.rs`):
   - Add the route:
     ```rust
     #[post("/<id>/change-password", data = "<payload>")]
     fn change_password(id: i32, payload: Json<ChangePasswordPayload>) -> UserResponse<String> {
         UserService::change_password(id, &payload.new_password)
             .map(|msg| Json(msg))
             .map_err(|err| error_response(Status::BadRequest, err))
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
| ✅ Define request/response DTOs inside `<name>_interface.rs`. | ❌ Don't pollute the controller with raw structs. |
