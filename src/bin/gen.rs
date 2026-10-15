use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn main() {
    println!("=========================================================");
    println!(" Rocket + Diesel Clean Architecture Generator (Cargo)");
    println!("=========================================================");

    let mut args: Vec<String> = env::args().collect();
    let table_name = if args.len() > 1 {
        let arg = args.remove(1);
        if arg == "--help" || arg == "-h" {
            println!("Usage: cargo gen <table_name>");
            println!("   or: cargo run --bin gen -- <table_name>\n");
            println!("Example:");
            println!("  cargo gen categories");
            println!("  cargo gen orders");
            return;
        }
        arg
    } else {
        print!("Enter table name (e.g. products, categories, orders): ");
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

    // Determine plural and singular
    let plural = table_name.clone();
    let singular = if plural.ends_with("ies") {
        format!("{}y", &plural[..plural.len() - 3])
    } else if plural.ends_with("shes") || plural.ends_with("ches") || plural.ends_with("xes") {
        plural[..plural.len() - 2].to_string()
    } else if plural.ends_with('s') {
        plural[..plural.len() - 1].to_string()
    } else {
        plural.clone()
    };
    let plural = if singular == table_name && !table_name.ends_with('s') {
        format!("{}s", singular)
    } else {
        plural
    };

    let singular_pascal = to_pascal_case(&singular);
    let plural_pascal = to_pascal_case(&plural);

    println!("\nTarget Entity:");
    println!("  Table Name:      {}", plural);
    println!("  Singular Entity: {} ({})", singular, singular_pascal);
    println!("  Plural Entity:   {} ({})", plural, plural_pascal);

    // 1. Run diesel migration if not already in schema.rs
    let schema_file = "src/database/schema.rs";
    let schema_content = fs::read_to_string(schema_file).unwrap_or_default();
    let table_pattern = format!("{} (id)", plural);

    if !schema_content.contains(&table_pattern) {
        println!("\n[1/6] Generating and applying Neon DB migration for '{}'...", plural);

        // Find diesel executable
        let diesel_bin = find_diesel();
        let status = Command::new(&diesel_bin)
            .args(["migration", "generate", &format!("create_{}", plural)])
            .status()
            .expect("Failed to execute diesel CLI");

        if !status.success() {
            eprintln!("Error running diesel migration generate");
            std::process::exit(1);
        }

        // Find created migration folder
        let mig_entry = fs::read_dir("migrations")
            .expect("Cannot read migrations dir")
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .filter(|e| e.file_name().to_string_lossy().contains(&format!("create_{}", plural)))
            .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

        if let Some(entry) = mig_entry {
            let mig_path = entry.path();
            let up_sql = format!(
                "CREATE TABLE {} (\n    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n    name VARCHAR NOT NULL,\n    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    created_by VARCHAR NOT NULL DEFAULT 'system',\n    updated_by VARCHAR NOT NULL DEFAULT 'system'\n);\n",
                plural
            );
            let down_sql = format!("DROP TABLE {};\n", plural);

            fs::write(mig_path.join("up.sql"), up_sql).expect("Failed to write up.sql");
            fs::write(mig_path.join("down.sql"), down_sql).expect("Failed to write down.sql");

            let run_status = Command::new(&diesel_bin)
                .args(["migration", "run"])
                .status()
                .expect("Failed to execute diesel migration run");

            if !run_status.success() {
                eprintln!("Error running diesel migration run");
                std::process::exit(1);
            }
            println!("  -> Migration applied to Neon DB and {} updated!", schema_file);
        }
    } else {
        println!("\n[1/6] Table '{}' already in {}, skipping migration.", plural, schema_file);
    }

    // 2. Append Models to src/database/models.rs
    let models_file = "src/database/models.rs";
    let models_content = fs::read_to_string(models_file).unwrap_or_default();
    if !models_content.contains(&format!("pub struct {}", singular_pascal)) {
        let model_code = format!(
            "\n\n// ==========================================\n// {} Models\n// ==========================================\nuse crate::database::schema::{};\n\n#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]\n#[diesel(table_name = {})]\n#[diesel(check_for_backend(diesel::pg::Pg))]\npub struct {} {{\n    pub id: uuid::Uuid,\n    pub name: String,\n    pub created_at: chrono::DateTime<chrono::Utc>,\n    pub updated_at: chrono::DateTime<chrono::Utc>,\n    pub created_by: String,\n    pub updated_by: String,\n}}\n\n#[derive(Insertable, Deserialize)]\n#[diesel(table_name = {})]\npub struct New{}<'a> {{\n    pub name: &'a str,\n}}\n",
            singular_pascal, plural, plural, singular_pascal, plural, singular_pascal
        );
        let mut file = OpenOptions::new().append(true).open(models_file).expect("Failed to open models.rs");
        file.write_all(model_code.as_bytes()).expect("Failed to write to models.rs");
        println!("[2/6] Appended {} to {}", singular_pascal, models_file);
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
        "use diesel::prelude::*;\nuse crate::database::connection::establish_connection;\nuse crate::database::models::{};\nuse crate::database::schema::{};\n\npub fn execute() -> Result<Vec<{}>, String> {{\n    let mut conn = establish_connection();\n    {}::table\n        .select({}::as_select())\n        .load(&mut conn)\n        .map_err(|e| e.to_string())\n}}\n",
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
        "use uuid::Uuid;\nuse crate::database::models::{};\nuse crate::usecases::{}::{{create_{}, delete_{}, get_all_{}, get_{}}};\n\npub struct {}Service;\n\nimpl {}Service {{\n    pub fn create_{}(name: &str) -> Result<{}, String> {{\n        create_{}::execute(name)\n    }}\n\n    pub fn get_{}_by_id(id: Uuid) -> Result<Option<{}>, String> {{\n        get_{}::execute(id)\n    }}\n\n    pub fn get_all_{}() -> Result<Vec<{}>, String> {{\n        get_all_{}::execute()\n    }}\n\n    pub fn delete_{}(id: Uuid) -> Result<usize, String> {{\n        delete_{}::execute(id)\n    }}\n}}\n",
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
    let interface_file = format!("src/controllers/{}_interface.rs", singular);
    let interface_code = format!(
        "use rocket::http::Status;\nuse rocket::response::status;\nuse rocket::serde::json::Json;\nuse serde::{{Deserialize, Serialize}};\n\n#[derive(Debug, Deserialize, Serialize)]\npub struct Create{}Payload {{\n    pub name: String,\n}}\n\npub type {}Response<T> = Result<Json<T>, status::Custom<String>>;\n\npub fn error_response(status: Status, message: String) -> status::Custom<String> {{\n    status::Custom(status, message)\n}}\n",
        singular_pascal, singular_pascal
    );
    fs::write(&interface_file, interface_code).expect("Failed to write interface");

    let controller_file = format!("src/controllers/{}_controller.rs", singular);
    let controller_code = format!(
        "use rocket::http::Status;\nuse rocket::serde::json::Json;\nuse rocket::{{delete, get, post, routes, Route}};\nuse uuid::Uuid;\nuse crate::controllers::{}_interface::{{error_response, Create{}Payload, {}Response}};\nuse crate::database::models::{};\nuse crate::services::{}_service::{}Service;\n\n#[get(\"/\")]\nfn list_{}() -> {}Response<Vec<{}>> {{\n    {}Service::get_all_{}()\n        .map(Json)\n        .map_err(|err| error_response(Status::InternalServerError, err))\n}}\n\n#[get(\"/<id>\")]\nfn get_{}(id: &str) -> {}Response<{}> {{\n    let parsed_id = match Uuid::parse_str(id) {{\n        Ok(u) => u,\n        Err(_) => return Err(error_response(Status::BadRequest, \"Invalid UUID format\".to_string())),\n    }};\n\n    match {}Service::get_{}_by_id(parsed_id) {{\n        Ok(Some(item)) => Ok(Json(item)),\n        Ok(None) => Err(error_response(Status::NotFound, \"{} not found\".to_string())),\n        Err(err) => Err(error_response(Status::InternalServerError, err)),\n    }}\n}}\n\n#[post(\"/\", data = \"<payload>\")]\nfn create_{}(payload: Json<Create{}Payload>) -> {}Response<{}> {{\n    {}Service::create_{}(&payload.name)\n        .map(Json)\n        .map_err(|err| error_response(Status::BadRequest, err))\n}}\n\n#[delete(\"/<id>\")]\nfn delete_{}(id: &str) -> Result<String, rocket::response::status::Custom<String>> {{\n    let parsed_id = match Uuid::parse_str(id) {{\n        Ok(u) => u,\n        Err(_) => return Err(error_response(Status::BadRequest, \"Invalid UUID format\".to_string())),\n    }};\n\n    match {}Service::delete_{}(parsed_id) {{\n        Ok(count) => Ok(format!(\"Deleted {{}} {}(s)\", count)),\n        Err(err) => Err(error_response(Status::InternalServerError, err)),\n    }}\n}}\n\npub fn get_routes() -> Vec<Route> {{\n    routes![list_{}, get_{}, create_{}, delete_{}]\n}}\n",
        singular, singular_pascal, singular_pascal,
        singular_pascal, singular, singular_pascal,
        plural, singular_pascal, singular_pascal,
        singular_pascal, plural,
        singular, singular_pascal, singular_pascal,
        singular_pascal, singular, singular_pascal,
        singular, singular_pascal, singular_pascal, singular_pascal,
        singular_pascal, singular,
        singular,
        singular_pascal, singular, singular,
        plural, singular, singular, singular
    );
    fs::write(&controller_file, controller_code).expect("Failed to write controller");

    let controllers_mod = "src/controllers/mod.rs";
    let mut controllers_mod_content = fs::read_to_string(controllers_mod).unwrap_or_default();
    let ctrl_decl = format!("pub mod {}_controller;\n", singular);
    let iface_decl = format!("pub mod {}_interface;\n", singular);
    if !controllers_mod_content.contains(&ctrl_decl) {
        controllers_mod_content.push_str(&ctrl_decl);
    }
    if !controllers_mod_content.contains(&iface_decl) {
        controllers_mod_content.push_str(&iface_decl);
    }
    fs::write(controllers_mod, controllers_mod_content).expect("Failed to update controllers/mod.rs");
    println!("[5/6] Created controller & interface for {}", singular);

    // 6. Mount in src/main.rs
    let main_file = "src/main.rs";
    let main_content = fs::read_to_string(main_file).unwrap_or_default();
    let mount_line = format!("        .mount(\"/{}\", controllers::{}_controller::get_routes())\n", plural, singular);

    if !main_content.contains(&format!("controllers::{}_controller::get_routes", singular)) {
        if let Some(pos) = main_content.rfind(".mount(") {
            if let Some(line_end) = main_content[pos..].find('\n') {
                let insert_at = pos + line_end + 1;
                let mut updated = main_content[..insert_at].to_string();
                updated.push_str(&mount_line);
                updated.push_str(&main_content[insert_at..]);
                fs::write(main_file, updated).expect("Failed to update main.rs");
                println!("[6/6] Mounted route '/{}' in {}", plural, main_file);
            }
        }
    }

    println!("\nSUCCESS! '{}' CRUD module generated completely via Cargo!", plural);
    println!("Routes available:");
    println!("  GET    /{}        -> List all {}", plural, plural);
    println!("  GET    /{}/<uuid> -> Get {} by UUID", plural, singular);
    println!("  POST   /{}        -> Create {} {{ \"name\": \"...\" }}", plural, singular);
    println!("  DELETE /{}/<uuid> -> Delete {} by UUID", plural, singular);
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
    // 1. Check PATH
    if Command::new("diesel").arg("--version").output().is_ok() {
        return "diesel".to_string();
    }

    // 2. Check ~/.cargo/bin
    if let Ok(home) = env::var("USERPROFILE").or_else(|_| env::var("HOME")) {
        let p = Path::new(&home).join(".cargo").join("bin").join(if cfg!(windows) { "diesel.exe" } else { "diesel" });
        if p.exists() {
            return p.to_string_lossy().to_string();
        }
    }

    "diesel".to_string()
}
