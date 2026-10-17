use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn main() {
    println!("=========================================================");
    println!(" Rocket + Diesel Architecture Generator (Cargo)");
    println!("=========================================================");

    let mut args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage:");
        println!("  cargo gen                  (Interactive menu: full CRUD, add route, or new module)");
        println!("  cargo gen <table_name>     (Directly generate full CRUD for a table)");
        println!("  cargo gen-api              (Directly generate individual API route)\n");
        println!("Example:");
        println!("  cargo gen categories");
        println!("  cargo gen");
        return;
    }

    let mode = if args.len() > 1 && !args[1].starts_with('-') {
        // Direct argument passed (e.g. cargo gen categories) -> Mode 1
        "1".to_string()
    } else {
        println!("What would you like to generate?");
        println!("  1. Full CRUD Module + DB Table (Neon migration, models, 4 usecases, service, controller) [default]");
        println!("  2. Add an API route to an EXISTING module (e.g. users, products)");
        println!("  3. Create a NEW module with that API route only (custom endpoint, no DB migration)");
        print!("Select [1/2/3] (default: 1): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        let trimmed = input.trim();
        if trimmed.is_empty() { "1".to_string() } else { trimmed.to_string() }
    };

    match mode.as_str() {
        "2" => generate_api_route(&mut args, false),
        "3" => generate_api_route(&mut args, true),
        _ => generate_full_crud(&mut args),
    }
}

// =========================================================================
// Mode 1: Full CRUD Module + DB Migration
// =========================================================================
fn generate_full_crud(args: &mut Vec<String>) {
    let table_name = if args.len() > 1 && !args[1].starts_with('-') {
        args.remove(1)
    } else {
        print!("Enter table name for full CRUD (e.g. products, categories, orders): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        input.trim().to_string()
    };

    let table_name = table_name.trim().to_lowercase();
    if table_name.is_empty() {
        eprintln!("Error: Table name cannot be empty.");
        std::process::exit(1);
    }

    // Ask if route should require authentication (default: Y)
    let is_auth = if args.iter().any(|a| a == "--public" || a == "--no-auth") {
        false
    } else if args.iter().any(|a| a == "--auth") {
        true
    } else {
        print!("Do you want this new route to require authentication? [Y/n] (default: Y): ");
        io::stdout().flush().unwrap();
        let mut auth_input = String::new();
        io::stdin().read_line(&mut auth_input).expect("Failed to read input");
        let trimmed = auth_input.trim().to_lowercase();
        !(trimmed == "n" || trimmed == "no")
    };

    let open_methods = if !is_auth {
        print!("Which methods should be open publicly?\n  1. GET only (read public, writes protected) [default]\n  2. ALL methods (GET, POST, DELETE, etc. all public)\nSelect [1/2] (default: 1): ");
        io::stdout().flush().unwrap();
        let mut m_input = String::new();
        io::stdin().read_line(&mut m_input).expect("Failed to read input");
        if m_input.trim() == "2" { "ALL" } else { "GET" }
    } else {
        "NONE"
    };

    let (singular, plural, singular_pascal, plural_pascal) = get_naming(&table_name);

    println!("\nTarget Entity:");
    println!("  Table Name:      {}", plural);
    println!("  Singular Entity: {} ({})", singular, singular_pascal);
    println!("  Plural Entity:   {} ({})", plural, plural_pascal);
    println!("  Auth Protected:  {}", if is_auth { "YES (default)" } else { "NO (public route)" });
    if !is_auth {
        println!("  Open Scope:      {}", if open_methods == "GET" { "GET only (reads public, writes protected)" } else { "ALL methods" });
    }

    // 1. Run diesel migration if not already in schema.rs
    let schema_file = "src/database/schema.rs";
    let schema_content = fs::read_to_string(schema_file).unwrap_or_default();

    if !schema_content.contains(&format!("diesel::table! {{\n    {} (id)", plural)) {
        println!("\n[1/6] Running Diesel Migration on Neon Postgres...");
        let diesel_bin = find_diesel();
        let migration_name = format!("create_{}", plural);
        let out = Command::new(&diesel_bin)
            .args(["migration", "generate", &migration_name])
            .output()
            .expect("Failed to run diesel migration generate");
        if !out.status.success() {
            eprintln!("Error generating migration: {}", String::from_utf8_lossy(&out.stderr));
            std::process::exit(1);
        }

        let mut migration_dir = None;
        if let Ok(entries) = fs::read_dir("migrations") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(&format!("_{}", migration_name)) {
                    migration_dir = Some(entry.path());
                }
            }
        }

        if let Some(dir) = migration_dir {
            let up_sql = format!(
                "CREATE TABLE {} (\n    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n    name VARCHAR NOT NULL,\n    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    created_by VARCHAR NOT NULL DEFAULT 'system',\n    updated_by VARCHAR NOT NULL DEFAULT 'system'\n);\n",
                plural
            );
            fs::write(dir.join("up.sql"), up_sql).expect("Failed to write up.sql");
            let down_sql = format!("DROP TABLE IF EXISTS {} CASCADE;\n", plural);
            fs::write(dir.join("down.sql"), down_sql).expect("Failed to write down.sql");
        }

        let run_out = Command::new(&diesel_bin)
            .args(["migration", "run"])
            .output()
            .expect("Failed to run diesel migration run");
        if !run_out.status.success() {
            eprintln!("Error applying migration: {}", String::from_utf8_lossy(&run_out.stderr));
            std::process::exit(1);
        }
        println!("[1/6] Diesel migration applied successfully!");
    } else {
        println!("\n[1/6] Table '{}' already in schema.rs. Skipping migration.", plural);
    }

    // 2. Append Model in src/database/models.rs
    let models_file = "src/database/models.rs";
    let models_content = fs::read_to_string(models_file).unwrap_or_default();
    if !models_content.contains(&format!("pub struct {}", singular_pascal)) {
        let model_code = format!(
            "\n// ==========================================\n// {} Models\n// ==========================================\nuse crate::database::schema::{};\n\n#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]\n#[diesel(table_name = {})]\n#[diesel(check_for_backend(diesel::pg::Pg))]\npub struct {} {{\n    pub id: uuid::Uuid,\n    pub name: String,\n    pub created_at: chrono::DateTime<chrono::Utc>,\n    pub updated_at: chrono::DateTime<chrono::Utc>,\n    pub created_by: String,\n    pub updated_by: String,\n}}\n\n#[derive(Insertable, Deserialize)]\n#[diesel(table_name = {})]\npub struct New{}<'a> {{\n    pub name: &'a str,\n}}\n",
            singular_pascal, plural, plural, singular_pascal, plural, singular_pascal
        );
        let mut file = OpenOptions::new().append(true).open(models_file).expect("Failed to open models.rs");
        file.write_all(model_code.as_bytes()).expect("Failed to append to models.rs");
        println!("[2/6] Appended model {} to {}", singular_pascal, models_file);
    }

    // 3. Generate Usecases
    let usecases_dir = format!("src/usecases/{}", singular);
    fs::create_dir_all(&usecases_dir).expect("Failed to create usecases dir");

    let create_usecase = format!(
        "use diesel::prelude::*;\nuse crate::database::connection::establish_connection;\nuse crate::database::models::{{{}, New{}}};\nuse crate::database::schema::{};\n\npub fn execute(name: &str) -> Result<{}, String> {{\n    if name.trim().is_empty() {{\n        return Err(\"Name cannot be empty\".to_string());\n    }}\n\n    let mut conn = establish_connection();\n    let new_record = New{} {{ name }};\n\n    diesel::insert_into({}::table)\n        .values(&new_record)\n        .returning({}::as_returning())\n        .get_result(&mut conn)\n        .map_err(|e| e.to_string())\n}}\n",
        singular_pascal, singular_pascal, plural, singular_pascal, singular_pascal, plural, singular_pascal
    );
    fs::write(format!("{}/create_{}.rs", usecases_dir, singular), create_usecase).expect("Failed to write create usecase");

    let get_usecase = format!(
        "use diesel::prelude::*;\nuse uuid::Uuid;\nuse crate::database::connection::establish_connection;\nuse crate::database::models::{};\nuse crate::database::schema::{};\n\npub fn execute(id: Uuid) -> Result<Option<{}>, String> {{\n    let mut conn = establish_connection();\n    {}::table\n        .find(id)\n        .select({}::as_select())\n        .first(&mut conn)\n        .optional()\n        .map_err(|e| e.to_string())\n}}\n",
        singular_pascal, plural, singular_pascal, plural, singular_pascal
    );
    fs::write(format!("{}/get_{}.rs", usecases_dir, singular), get_usecase).expect("Failed to write get usecase");

    let get_all_usecase = format!(
        "use diesel::prelude::*;\nuse crate::database::connection::establish_connection;\nuse crate::database::models::{};\nuse crate::database::schema::{};\n\npub fn execute(limit: Option<i64>, offset: Option<i64>) -> Result<Vec<{}>, String> {{\n    let mut conn = establish_connection();\n    let mut query = {}::table.into_boxed();\n    if let Some(l) = limit {{\n        query = query.limit(l);\n    }}\n    if let Some(o) = offset {{\n        query = query.offset(o);\n    }}\n    query\n        .select({}::as_select())\n        .load(&mut conn)\n        .map_err(|e| e.to_string())\n}}\n",
        singular_pascal, plural, singular_pascal, plural, singular_pascal
    );
    fs::write(format!("{}/get_all_{}.rs", usecases_dir, plural), get_all_usecase).expect("Failed to write get_all usecase");

    let delete_usecase = format!(
        "use diesel::prelude::*;\nuse uuid::Uuid;\nuse crate::database::connection::establish_connection;\nuse crate::database::schema::{};\n\npub fn execute(id: Uuid) -> Result<usize, String> {{\n    let mut conn = establish_connection();\n    diesel::delete({}::table.find(id))\n        .execute(&mut conn)\n        .map_err(|e| e.to_string())\n}}\n",
        plural, plural
    );
    fs::write(format!("{}/delete_{}.rs", usecases_dir, singular), delete_usecase).expect("Failed to write delete usecase");

    let usecase_mod = format!(
        "pub mod create_{};\npub mod delete_{};\npub mod get_all_{};\npub mod get_{};\n",
        singular, singular, plural, singular
    );
    fs::write(format!("{}/mod.rs", usecases_dir), usecase_mod).expect("Failed to write usecase mod.rs");

    let root_usecase_mod = "src/usecases/mod.rs";
    let root_usecase_content = fs::read_to_string(root_usecase_mod).unwrap_or_default();
    let mod_decl = format!("pub mod {};\n", singular);
    if !root_usecase_content.contains(&mod_decl) {
        let mut file = OpenOptions::new().append(true).open(root_usecase_mod).expect("Failed to open usecases/mod.rs");
        file.write_all(mod_decl.as_bytes()).expect("Failed to update usecases/mod.rs");
    }
    println!("[3/6] Created usecases under {}/", usecases_dir);

    // 4. Generate Service
    let service_file = format!("src/services/{}_service.rs", singular);
    let service_code = format!(
        "use uuid::Uuid;\nuse crate::database::models::{};\nuse crate::usecases::{}::{{create_{}, delete_{}, get_all_{}, get_{}}};\n\npub struct {}Service;\n\nimpl {}Service {{\n    pub fn create_{}(name: &str) -> Result<{}, String> {{\n        create_{}::execute(name)\n    }}\n\n    pub fn get_{}_by_id(id: Uuid) -> Result<Option<{}>, String> {{\n        get_{}::execute(id)\n    }}\n\n    pub fn get_all_{}(limit: Option<i64>, offset: Option<i64>) -> Result<Vec<{}>, String> {{\n        get_all_{}::execute(limit, offset)\n    }}\n\n    pub fn delete_{}(id: Uuid) -> Result<usize, String> {{\n        delete_{}::execute(id)\n    }}\n}}\n",
        singular_pascal, singular, singular, singular, plural, singular,
        singular_pascal, singular_pascal, singular, singular_pascal, singular,
        singular, singular_pascal, singular, plural, singular_pascal, plural,
        singular, singular
    );
    fs::write(&service_file, service_code).expect("Failed to write service");

    let services_mod = "src/services/mod.rs";
    let services_mod_content = fs::read_to_string(services_mod).unwrap_or_default();
    let service_decl = format!("pub mod {}_service;\n", singular);
    if !services_mod_content.contains(&service_decl) {
        let mut file = OpenOptions::new().append(true).open(services_mod).expect("Failed to open services/mod.rs");
        file.write_all(service_decl.as_bytes()).expect("Failed to update services/mod.rs");
    }
    println!("[4/6] Created service {}", service_file);

    // 5. Generate Interface & Controller
    let controllers_dir = format!("src/controllers/{}", plural);
    fs::create_dir_all(&controllers_dir).expect("Failed to create controllers dir");

    let interface_file = format!("{}/{}.interface.rs", controllers_dir, singular);
    let interface_code = format!(
        "use rocket::http::Status;\nuse rocket::response::status;\nuse rocket::serde::json::Json;\nuse serde::{{Deserialize, Serialize}};\nuse crate::database::models::{};\n\n// --- DTOs / Payloads ---\n#[derive(Debug, Deserialize, Serialize)]\npub struct Create{}Payload {{\n    pub name: String,\n}}\n\npub type {}Response<T> = Result<Json<T>, status::Custom<String>>;\n\npub fn error_response(status: Status, message: String) -> status::Custom<String> {{\n    status::Custom(status, message)\n}}\n\n// --- Controller Interface Contract ---\npub trait I{}ControllerInterface {{\n    fn list_{}(limit: Option<i64>, offset: Option<i64>) -> {}Response<Vec<{}>>;\n    fn get_{}(id: &str) -> {}Response<{}>;\n    fn create_{}(payload: Json<Create{}Payload>) -> {}Response<{}>;\n    fn delete_{}(id: &str) -> Result<String, status::Custom<String>>;\n}}\n",
        singular_pascal,
        singular_pascal,
        singular_pascal,
        singular_pascal,
        plural, singular_pascal, singular_pascal,
        singular, singular_pascal, singular_pascal,
        singular, singular_pascal, singular_pascal, singular_pascal,
        singular
    );
    fs::write(&interface_file, interface_code).expect("Failed to write interface");

    let controller_file = format!("{}/{}.controller.rs", controllers_dir, singular);
    let controller_code = format!(
        "use rocket::http::Status;\nuse rocket::serde::json::Json;\nuse rocket::{{delete, get, post, routes, Route}};\nuse uuid::Uuid;\nuse super::interface::{{error_response, Create{}Payload, I{}ControllerInterface, {}Response}};\nuse crate::database::models::{};\nuse crate::services::{}_service::{}Service;\n\npub struct {}Controller;\n\nimpl I{}ControllerInterface for {}Controller {{\n    fn list_{}(limit: Option<i64>, offset: Option<i64>) -> {}Response<Vec<{}>> {{\n        {}Service::get_all_{}(limit, offset)\n            .map(Json)\n            .map_err(|err| error_response(Status::InternalServerError, err))\n    }}\n\n    fn get_{}(id: &str) -> {}Response<{}> {{\n        let parsed_id = match Uuid::parse_str(id) {{\n            Ok(u) => u,\n            Err(_) => return Err(error_response(Status::BadRequest, \"Invalid UUID format\".to_string())),\n        }};\n\n        match {}Service::get_{}_by_id(parsed_id) {{\n            Ok(Some(item)) => Ok(Json(item)),\n            Ok(None) => Err(error_response(Status::NotFound, \"{} not found\".to_string())),\n            Err(err) => Err(error_response(Status::InternalServerError, err)),\n        }}\n    }}\n\n    fn create_{}(payload: Json<Create{}Payload>) -> {}Response<{}> {{\n        {}Service::create_{}(&payload.name)\n            .map(Json)\n            .map_err(|err| error_response(Status::BadRequest, err))\n    }}\n\n    fn delete_{}(id: &str) -> Result<String, rocket::response::status::Custom<String>> {{\n        let parsed_id = match Uuid::parse_str(id) {{\n            Ok(u) => u,\n            Err(_) => return Err(error_response(Status::BadRequest, \"Invalid UUID format\".to_string())),\n        }};\n\n        match {}Service::delete_{}(parsed_id) {{\n            Ok(count) => Ok(format!(\"Deleted {{}} {}(s)\", count)),\n            Err(err) => Err(error_response(Status::InternalServerError, err)),\n        }}\n    }}\n}}\n\n// --- Route Handlers ---\n#[get(\"/?<limit>&<offset>\")]\nfn list_{}(limit: Option<i64>, offset: Option<i64>) -> {}Response<Vec<{}>> {{\n    {}Controller::list_{}(limit, offset)\n}}\n\n#[get(\"/<id>\")]\nfn get_{}(id: &str) -> {}Response<{}> {{\n    {}Controller::get_{}(id)\n}}\n\n#[post(\"/\", data = \"<payload>\")]\nfn create_{}(payload: Json<Create{}Payload>) -> {}Response<{}> {{\n    {}Controller::create_{}(payload)\n}}\n\n#[delete(\"/<id>\")]\nfn delete_{}(id: &str) -> Result<String, rocket::response::status::Custom<String>> {{\n    {}Controller::delete_{}(id)\n}}\n\npub fn get_routes() -> Vec<Route> {{\n    routes![list_{}, get_{}, create_{}, delete_{}]\n}}\n",
        singular_pascal, singular_pascal, singular_pascal,
        singular_pascal, singular, singular_pascal,
        singular_pascal, singular_pascal, singular_pascal,
        plural, singular_pascal, singular_pascal,
        singular_pascal, plural,
        singular, singular_pascal, singular_pascal,
        singular_pascal, singular, singular_pascal,
        singular, singular_pascal, singular_pascal, singular_pascal,
        singular_pascal, singular,
        singular,
        singular_pascal, singular, singular,
        // Route handlers:
        plural, singular_pascal, singular_pascal, singular_pascal, plural,
        singular, singular_pascal, singular_pascal, singular_pascal, singular,
        singular, singular_pascal, singular_pascal, singular_pascal, singular_pascal, singular,
        singular, singular_pascal, singular,
        plural, singular, singular, singular
    );
    fs::write(&controller_file, controller_code).expect("Failed to write controller");

    let submod_file = format!("{}/mod.rs", controllers_dir);
    let submod_code = format!(
        "#[path = \"{}.controller.rs\"]\npub mod controller;\n\n#[path = \"{}.interface.rs\"]\npub mod interface;\n",
        singular, singular
    );
    fs::write(&submod_file, submod_code).expect("Failed to write controller mod.rs");

    let controllers_mod = "src/controllers/mod.rs";
    let mut controllers_mod_content = fs::read_to_string(controllers_mod).unwrap_or_default();
    let mod_decl = format!("pub mod {};\n", plural);
    if !controllers_mod_content.contains(&mod_decl) {
        controllers_mod_content.push_str(&mod_decl);
    }
    fs::write(controllers_mod, controllers_mod_content).expect("Failed to update controllers/mod.rs");
    println!("[5/6] Created controller & interface for {} under {}/", singular, controllers_dir);

    // 6. Mount in src/main.rs
    let main_file = "src/main.rs";
    let mut main_content = fs::read_to_string(main_file).unwrap_or_default();

    // If public route was chosen (not auth), register in PublicRoutes
    if !is_auth {
        let prefix_pattern = if open_methods == "GET" {
            format!("GET /{}/*", plural)
        } else {
            format!("/{}/*", plural)
        };
        if !main_content.contains(&prefix_pattern) {
            let open_call = format!("        .open(\"{}\")", prefix_pattern);
            if main_content.contains("let public_routes = PublicRoutes::empty();") {
                let replaced = format!("let public_routes = PublicRoutes::empty()\n{};", open_call);
                main_content = main_content.replacen("let public_routes = PublicRoutes::empty();", &replaced, 1);
            } else if let Some(pos) = main_content.find("let public_routes = PublicRoutes::empty()") {
                let insert_at = pos + "let public_routes = PublicRoutes::empty()".len();
                main_content.insert_str(insert_at, &format!("\n{}", open_call));
            }
        }
    }

    let mount_line = format!("        .mount(\"/{}\", controllers::{}::controller::get_routes())\n", plural, plural);

    if !main_content.contains(&format!("controllers::{}::controller::get_routes", plural)) {
        if let Some(pos) = main_content.rfind(".mount(") {
            if let Some(line_end) = main_content[pos..].find('\n') {
                let insert_at = pos + line_end + 1;
                let mut updated = main_content[..insert_at].to_string();
                updated.push_str(&mount_line);
                updated.push_str(&main_content[insert_at..]);
                main_content = updated;
            }
        }
    }
    fs::write(main_file, main_content).expect("Failed to update main.rs");
    println!(
        "[6/6] Mounted route '/{}' in {} ({})",
        plural,
        main_file,
        if is_auth { "Protected by AuthFairing" } else { "Configured in PublicRoutes" }
    );

    println!("\nSUCCESS! '{}' CRUD module generated completely via Cargo!", plural);
    println!("Routes available:");
    println!("  GET    /{}        -> List all {}", plural, plural);
    println!("  GET    /{}/<uuid> -> Get {} by UUID", plural, singular);
    println!("  POST   /{}        -> Create {} {{ \"name\": \"...\" }}", plural, singular);
    println!("  DELETE /{}/<uuid> -> Delete {} by UUID", plural, singular);
}

// =========================================================================
// Modes 2 & 3: API Route Generator (Existing module or New custom module)
// =========================================================================
fn generate_api_route(_args: &mut Vec<String>, is_new_module: bool) {
    let existing_modules = get_existing_modules();

    let domain = if is_new_module {
        print!("Enter name for NEW module (e.g. analytics, auth, reports): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        input.trim().to_string()
    } else {
        println!("Existing modules found: {}", existing_modules.join(", "));
        let default_domain = existing_modules.first().cloned().unwrap_or_else(|| "users".to_string());
        print!("Select target module (default: {}): ", default_domain);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        let trimmed = input.trim();
        if trimmed.is_empty() { default_domain } else { trimmed.to_string() }
    };

    let domain = domain.trim().to_lowercase();
    if domain.is_empty() {
        eprintln!("Error: Module name cannot be empty.");
        std::process::exit(1);
    }

    let (singular, plural, singular_pascal, _plural_pascal) = get_naming(&domain);

    // Action Name
    print!("Enter action / function name (e.g. change_password, search, summary): ");
    io::stdout().flush().unwrap();
    let mut action_input = String::new();
    io::stdin().read_line(&mut action_input).expect("Failed to read input");
    let action = action_input.trim().to_lowercase();
    if action.is_empty() {
        eprintln!("Error: Action name cannot be empty.");
        std::process::exit(1);
    }

    // HTTP Method
    print!("HTTP Method [GET, POST, PUT, DELETE, PATCH] (default: GET): ");
    io::stdout().flush().unwrap();
    let mut method_input = String::new();
    io::stdin().read_line(&mut method_input).expect("Failed to read input");
    let method = method_input.trim().to_uppercase();
    let method = if method.is_empty() { "GET".to_string() } else { method };
    let method_lower = method.to_lowercase();

    // Route Subpath
    let default_path = format!("/{}", action.replace('_', "-"));
    print!("Endpoint subpath under /{} (default: {}): ", plural, default_path);
    io::stdout().flush().unwrap();
    let mut path_input = String::new();
    io::stdin().read_line(&mut path_input).expect("Failed to read input");
    let path_val = path_input.trim().to_string();
    let route_path = if path_val.is_empty() { default_path } else { path_val };
    let route_path = if route_path.starts_with('/') { route_path } else { format!("/{}", route_path) };

    // Auth Prompt
    print!("Do you want this route to require authentication? [Y/n] (default: Y): ");
    io::stdout().flush().unwrap();
    let mut auth_input = String::new();
    io::stdin().read_line(&mut auth_input).expect("Failed to read input");
    let is_auth = !(auth_input.trim().eq_ignore_ascii_case("n") || auth_input.trim().eq_ignore_ascii_case("no"));

    // Open Method Prompt
    let open_mode = if !is_auth {
        print!(
            "Which method(s) should be open publicly?\n  1. {} only ({} /{}{} is public) [default]\n  2. ALL methods for /{}{}\nSelect [1/2] (default: 1): ",
            method, method, plural, route_path, plural, route_path
        );
        io::stdout().flush().unwrap();
        let mut m_input = String::new();
        io::stdin().read_line(&mut m_input).expect("Failed to read input");
        if m_input.trim() == "2" { "ALL" } else { "THIS" }
    } else {
        "NONE"
    };

    println!("\nGenerating API Route:");
    println!("  Module:       {}", plural);
    println!("  Action:       {}", action);
    println!("  HTTP Endpoint: {} /{}{}", method, plural, route_path);
    println!("  Auth:         {}", if is_auth { "YES (Protected by default)" } else { "NO (Public Route)" });
    if !is_auth {
        println!("  Open Scope:   {}", if open_mode == "THIS" { format!("{} only", method) } else { "ALL methods".to_string() });
    }

    // 1. Usecase
    let usecase_dir = format!("src/usecases/{}", singular);
    fs::create_dir_all(&usecase_dir).expect("Failed to create usecase dir");
    let usecase_file = format!("{}/{}.rs", usecase_dir, action);
    if !Path::new(&usecase_file).exists() {
        let usecase_code = format!(
            "// Usecase: {}\npub fn execute() -> Result<String, String> {{\n    // TODO: Implement business/query logic here\n    Ok(\"{} executed successfully\".to_string())\n}}\n",
            action, action
        );
        fs::write(&usecase_file, usecase_code).expect("Failed to write usecase file");
        println!("[1/5] Created usecase: {}", usecase_file);
    }

    let usecase_mod = format!("{}/mod.rs", usecase_dir);
    let mut usecase_mod_content = fs::read_to_string(&usecase_mod).unwrap_or_default();
    let mod_line = format!("pub mod {};\n", action);
    if !usecase_mod_content.contains(&mod_line) {
        usecase_mod_content.push_str(&mod_line);
        fs::write(&usecase_mod, usecase_mod_content).expect("Failed to update usecase mod.rs");
    }

    let root_usecase_mod = "src/usecases/mod.rs";
    let mut root_usecase_content = fs::read_to_string(root_usecase_mod).unwrap_or_default();
    let domain_mod = format!("pub mod {};\n", singular);
    if !root_usecase_content.contains(&domain_mod) {
        root_usecase_content.push_str(&domain_mod);
        fs::write(root_usecase_mod, root_usecase_content).expect("Failed to update root usecases/mod.rs");
    }

    // 2. Service
    let service_file = format!("src/services/{}_service.rs", singular);
    let mut service_content = if Path::new(&service_file).exists() {
        fs::read_to_string(&service_file).unwrap_or_default()
    } else {
        format!(
            "pub struct {}Service;\n\nimpl {}Service {{\n}}\n",
            singular_pascal, singular_pascal
        )
    };

    let service_method = format!("pub fn {}(", action);
    if !service_content.contains(&service_method) {
        let method_code = format!(
            "    pub fn {}() -> Result<String, String> {{\n        crate::usecases::{}::{}::execute()\n    }}\n",
            action, singular, action
        );
        if let Some(pos) = service_content.rfind('}') {
            service_content.insert_str(pos, &method_code);
            fs::write(&service_file, service_content).expect("Failed to update service");
            println!("[2/5] Added method to service: {}", service_file);
        }
    }

    let services_mod = "src/services/mod.rs";
    let mut services_mod_content = fs::read_to_string(services_mod).unwrap_or_default();
    let svc_decl = format!("pub mod {}_service;\n", singular);
    if !services_mod_content.contains(&svc_decl) {
        services_mod_content.push_str(&svc_decl);
        fs::write(services_mod, services_mod_content).expect("Failed to update services/mod.rs");
    }

    // 3. Interface
    let controller_dir = format!("src/controllers/{}", plural);
    fs::create_dir_all(&controller_dir).expect("Failed to create controller dir");
    let interface_file = format!("{}/{}.interface.rs", controller_dir, singular);

    let trait_name = format!("I{}ControllerInterface", singular_pascal);
    let mut iface_content = if Path::new(&interface_file).exists() {
        fs::read_to_string(&interface_file).unwrap_or_default()
    } else {
        format!(
            "use rocket::http::Status;\nuse rocket::response::status;\nuse rocket::serde::json::Json;\nuse serde::{{Deserialize, Serialize}};\n\npub type {}Response<T> = Result<Json<T>, status::Custom<String>>;\n\npub fn error_response(status: Status, message: String) -> status::Custom<String> {{\n    status::Custom(status, message)\n}}\n\npub trait {} {{\n}}\n",
            singular_pascal, trait_name
        )
    };

    let trait_method_decl = format!("fn {}() -> {}Response<String>;", action, singular_pascal);
    if !iface_content.contains(&format!("fn {}()", action)) {
        if let Some(pos) = iface_content.rfind(&trait_name) {
            if let Some(brace) = iface_content[pos..].find('{') {
                let insert_at = pos + brace + 1;
                iface_content.insert_str(insert_at, &format!("\n    {}\n", trait_method_decl));
                fs::write(&interface_file, iface_content).expect("Failed to update interface");
                println!("[3/5] Declared method in interface trait: {}", interface_file);
            }
        }
    }

    // 4. Controller
    let controller_file = format!("{}/{}.controller.rs", controller_dir, singular);
    let mut ctrl_content = if Path::new(&controller_file).exists() {
        fs::read_to_string(&controller_file).unwrap_or_default()
    } else {
        format!(
            "use rocket::http::Status;\nuse rocket::serde::json::Json;\nuse rocket::{{{}}};\nuse rocket::{{routes, Route}};\nuse super::interface::{{error_response, I{}, {}Response}};\nuse crate::services::{}_service::{}Service;\n\npub struct {}Controller;\n\nimpl I{} for {}Controller {{\n}}\n\npub fn get_routes() -> Vec<Route> {{\n    routes![]\n}}\n",
            method_lower, trait_name, format!("{}Response", singular_pascal),
            singular, singular_pascal,
            singular_pascal, trait_name, singular_pascal
        )
    };

    if !ctrl_content.contains(&method_lower) {
        if let Some(pos) = ctrl_content.find("use rocket::{") {
            let insert_at = pos + "use rocket::{".len();
            ctrl_content.insert_str(insert_at, &format!("{}, ", method_lower));
        }
    }

    if !ctrl_content.contains(&format!("fn {}()", action)) {
        let impl_decl = format!("impl I{} for {}Controller {{", singular_pascal, singular_pascal);
        let impl_code = format!(
            "\n    fn {}() -> {}Response<String> {{\n        {}Service::{}()\n            .map(Json)\n            .map_err(|err| error_response(Status::BadRequest, err))\n    }}\n",
            action, singular_pascal, singular_pascal, action
        );

        if let Some(pos) = ctrl_content.find(&impl_decl) {
            let insert_at = pos + impl_decl.len();
            ctrl_content.insert_str(insert_at, &impl_code);
        }

        let handler_code = format!(
            "\n#[{}(\"{}\")]\nfn {}() -> {}Response<String> {{\n    {}Controller::{}()\n}}\n",
            method_lower, route_path, action, singular_pascal, singular_pascal, action
        );

        if let Some(pos) = ctrl_content.find("pub fn get_routes()") {
            ctrl_content.insert_str(pos, &handler_code);
        }

        if let Some(pos) = ctrl_content.find("routes![") {
            let insert_at = pos + "routes![".len();
            if ctrl_content[insert_at..].starts_with(']') {
                ctrl_content.insert_str(insert_at, &action);
            } else {
                ctrl_content.insert_str(insert_at, &format!("{}, ", action));
            }
        }

        fs::write(&controller_file, ctrl_content).expect("Failed to update controller");
        println!("[4/5] Implemented handler & route in controller: {}", controller_file);
    }

    let submod_file = format!("{}/mod.rs", controller_dir);
    if !Path::new(&submod_file).exists() {
        let submod_code = format!(
            "#[path = \"{}.controller.rs\"]\npub mod controller;\n\n#[path = \"{}.interface.rs\"]\npub mod interface;\n",
            singular, singular
        );
        fs::write(&submod_file, submod_code).expect("Failed to write controller mod.rs");
    }

    let controllers_mod = "src/controllers/mod.rs";
    let mut controllers_mod_content = fs::read_to_string(controllers_mod).unwrap_or_default();
    let mod_decl = format!("pub mod {};\n", plural);
    if !controllers_mod_content.contains(&mod_decl) {
        controllers_mod_content.push_str(&mod_decl);
        fs::write(controllers_mod, controllers_mod_content).expect("Failed to update controllers/mod.rs");
    }

    // 5. Mount & PublicRoutes in main.rs
    let main_file = "src/main.rs";
    let mut main_content = fs::read_to_string(main_file).unwrap_or_default();

    if !is_auth {
        let public_rule = if open_mode == "THIS" {
            format!("{} /{}{}", method, plural, route_path)
        } else {
            format!("/{}{}/*", plural, route_path)
        };

        if !main_content.contains(&public_rule) {
            let open_call = format!("        .open(\"{}\")", public_rule);
            if main_content.contains("let public_routes = PublicRoutes::empty();") {
                let replaced = format!("let public_routes = PublicRoutes::empty()\n{};", open_call);
                main_content = main_content.replacen("let public_routes = PublicRoutes::empty();", &replaced, 1);
            } else if let Some(pos) = main_content.find("let public_routes = PublicRoutes::empty()") {
                let insert_at = pos + "let public_routes = PublicRoutes::empty()".len();
                main_content.insert_str(insert_at, &format!("\n{}", open_call));
            }
        }
    }

    let mount_signature = format!("controllers::{}::controller::get_routes()", plural);
    let mount_line = format!("        .mount(\"/{}\", controllers::{}::controller::get_routes())\n", plural, plural);
    if !main_content.contains(&mount_signature) {
        if let Some(pos) = main_content.rfind(".mount(") {
            if let Some(line_end) = main_content[pos..].find('\n') {
                let insert_at = pos + line_end + 1;
                let mut updated = main_content[..insert_at].to_string();
                updated.push_str(&mount_line);
                updated.push_str(&main_content[insert_at..]);
                main_content = updated;
            }
        }
    }

    fs::write(main_file, main_content).expect("Failed to update main.rs");
    println!(
        "[5/5] Updated main.rs: route mounted & auth configured ({})",
        if is_auth { "Protected by AuthFairing" } else { "Registered in PublicRoutes" }
    );

    println!("\nSUCCESS! API route generated cleanly:");
    println!("  HTTP:   {} /{}{}", method, plural, route_path);
    println!("  Auth:   {}", if is_auth { "Protected (Auth required)" } else { "Open (Public)" });
}

// Helpers
fn get_existing_modules() -> Vec<String> {
    let mut modules = Vec::new();
    if let Ok(entries) = fs::read_dir("src/controllers") {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    modules.push(name.to_string());
                }
            }
        }
    }
    modules.sort();
    modules
}

fn get_naming(name: &str) -> (String, String, String, String) {
    let plural = name.to_string();
    let singular = if plural.ends_with("ies") {
        format!("{}y", &plural[..plural.len() - 3])
    } else if plural.ends_with("shes") || plural.ends_with("ches") || plural.ends_with("xes") {
        plural[..plural.len() - 2].to_string()
    } else if plural.ends_with('s') {
        plural[..plural.len() - 1].to_string()
    } else {
        plural.clone()
    };
    let plural = if singular == name && !name.ends_with('s') {
        format!("{}s", singular)
    } else {
        plural
    };
    let singular_pascal = to_pascal_case(&singular);
    let plural_pascal = to_pascal_case(&plural);
    (singular, plural, singular_pascal, plural_pascal)
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
            }
        })
        .collect()
}

fn find_diesel() -> String {
    if Command::new("diesel").arg("--version").output().is_ok() {
        return "diesel".to_string();
    }
    if let Ok(home) = env::var("USERPROFILE").or_else(|_| env::var("HOME")) {
        let p = Path::new(&home).join(".cargo").join("bin").join(if cfg!(windows) { "diesel.exe" } else { "diesel" });
        if p.exists() {
            return p.to_string_lossy().to_string();
        }
    }
    "diesel".to_string()
}
