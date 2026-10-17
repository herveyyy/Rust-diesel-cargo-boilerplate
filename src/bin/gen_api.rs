use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

fn main() {
    println!("=========================================================");
    println!(" Rocket + Diesel API Route Generator");
    println!("=========================================================");

    let mut args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: cargo gen-api [domain] [action] [method] [path]");
        println!("   or: cargo run --bin gen_api\n");
        println!("Example:");
        println!("  cargo gen-api users change_password POST /<id>/change-password");
        println!("  cargo gen-api products search GET /search");
        return;
    }

    // 1. Target Domain
    let domain = if args.len() > 1 {
        args.remove(1)
    } else {
        print!("Target domain/entity (e.g. users, products, orders): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        input.trim().to_string()
    };
    let domain = domain.trim().to_lowercase();
    if domain.is_empty() {
        eprintln!("Error: Domain cannot be empty.");
        std::process::exit(1);
    }

    // Singular / Plural
    let plural = domain.clone();
    let singular = if plural.ends_with("ies") {
        format!("{}y", &plural[..plural.len() - 3])
    } else if plural.ends_with("shes") || plural.ends_with("ches") || plural.ends_with("xes") {
        plural[..plural.len() - 2].to_string()
    } else if plural.ends_with('s') {
        plural[..plural.len() - 1].to_string()
    } else {
        plural.clone()
    };
    let plural = if singular == domain && !domain.ends_with('s') {
        format!("{}s", singular)
    } else {
        plural
    };

    let singular_pascal = to_pascal_case(&singular);

    // 2. Action Name
    let action = if args.len() > 1 {
        args.remove(1)
    } else {
        print!("Action / function name (e.g. change_password, search, summary): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        input.trim().to_string()
    };
    let action = action.trim().to_lowercase();
    if action.is_empty() {
        eprintln!("Error: Action name cannot be empty.");
        std::process::exit(1);
    }
    let _action_pascal = to_pascal_case(&action);

    // 3. HTTP Method
    let method = if args.len() > 1 {
        args.remove(1).to_uppercase()
    } else {
        print!("HTTP Method [GET, POST, PUT, DELETE, PATCH] (default: GET): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        let m = input.trim().to_uppercase();
        if m.is_empty() { "GET".to_string() } else { m }
    };
    let method_lower = method.to_lowercase();

    // 4. Route Subpath
    let route_path = if args.len() > 1 {
        args.remove(1)
    } else {
        let default_path = format!("/{}", action.replace('_', "-"));
        print!("Endpoint subpath under /{} (default: {}): ", plural, default_path);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        let p = input.trim().to_string();
        if p.is_empty() { default_path } else { p }
    };
    let route_path = if route_path.starts_with('/') {
        route_path
    } else {
        format!("/{}", route_path)
    };

    // 5. Authentication Prompt
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

    // 6. Which method is open?
    let open_mode = if !is_auth {
        print!(
            "Which method(s) should be open publicly?\n  1. {} only ({} /{}{} is public) [default]\n  2. ALL methods for /{}{} (any HTTP verb)\nSelect [1/2] (default: 1): ",
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
    println!("  Domain:       {}", plural);
    println!("  Action:       {}", action);
    println!("  HTTP Endpoint: {} /{}{}", method, plural, route_path);
    println!("  Auth:         {}", if is_auth { "YES (Protected by default)" } else { "NO (Public Route)" });
    if !is_auth {
        println!("  Open Scope:   {}", if open_mode == "THIS" { format!("{} only", method) } else { "ALL methods".to_string() });
    }
    println!();

    // -------------------------------------------------------------
    // 1. Create Usecase
    // -------------------------------------------------------------
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
    } else {
        println!("[1/5] Usecase already exists: {}", usecase_file);
    }

    // Register in usecases/<singular>/mod.rs
    let usecase_mod = format!("{}/mod.rs", usecase_dir);
    let mut usecase_mod_content = fs::read_to_string(&usecase_mod).unwrap_or_default();
    let mod_line = format!("pub mod {};\n", action);
    if !usecase_mod_content.contains(&mod_line) {
        usecase_mod_content.push_str(&mod_line);
        fs::write(&usecase_mod, usecase_mod_content).expect("Failed to update usecase mod.rs");
    }

    // Register domain in src/usecases/mod.rs
    let root_usecase_mod = "src/usecases/mod.rs";
    let mut root_usecase_content = fs::read_to_string(root_usecase_mod).unwrap_or_default();
    let domain_mod = format!("pub mod {};\n", singular);
    if !root_usecase_content.contains(&domain_mod) {
        root_usecase_content.push_str(&domain_mod);
        fs::write(root_usecase_mod, root_usecase_content).expect("Failed to update root usecases/mod.rs");
    }

    // -------------------------------------------------------------
    // 2. Service
    // -------------------------------------------------------------
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
    } else {
        println!("[2/5] Method already exists in service: {}", service_file);
    }

    // Register in src/services/mod.rs
    let services_mod = "src/services/mod.rs";
    let mut services_mod_content = fs::read_to_string(services_mod).unwrap_or_default();
    let svc_decl = format!("pub mod {}_service;\n", singular);
    if !services_mod_content.contains(&svc_decl) {
        services_mod_content.push_str(&svc_decl);
        fs::write(services_mod, services_mod_content).expect("Failed to update services/mod.rs");
    }

    // -------------------------------------------------------------
    // 3. Interface (Trait + DTO)
    // -------------------------------------------------------------
    let controller_dir = format!("src/controllers/{}", plural);
    fs::create_dir_all(&controller_dir).expect("Failed to create controller dir");
    let interface_file = format!("{}/{}.interface.rs", controller_dir, singular);

    let mut iface_content = if Path::new(&interface_file).exists() {
        fs::read_to_string(&interface_file).unwrap_or_default()
    } else {
        format!(
            "use rocket::http::Status;\nuse rocket::response::status;\nuse rocket::serde::json::Json;\nuse serde::{{Deserialize, Serialize}};\n\npub type {}Response<T> = Result<Json<T>, status::Custom<String>>;\n\npub fn error_response(status: Status, message: String) -> status::Custom<String> {{\n    status::Custom(status, message)\n}}\n\npub trait I{}ControllerInterface {{\n}}\n",
            singular_pascal, singular_pascal
        )
    };

    let trait_name = format!("I{}ControllerInterface", singular_pascal);
    let trait_method_decl = format!("fn {}() -> {}Response<String>;", action, singular_pascal);

    if !iface_content.contains(&format!("fn {}()", action)) {
        if let Some(pos) = iface_content.rfind(&trait_name) {
            if let Some(brace) = iface_content[pos..].find('{') {
                let insert_at = pos + brace + 1;
                iface_content.insert_str(insert_at, &format!("\n    {}\n", trait_method_decl));
                fs::write(&interface_file, iface_content).expect("Failed to update interface");
                println!("[3/5] Declared trait method in interface: {}", interface_file);
            }
        }
    } else {
        println!("[3/5] Trait method already in interface: {}", interface_file);
    }

    // -------------------------------------------------------------
    // 4. Controller
    // -------------------------------------------------------------
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

    // Ensure Rocket HTTP verb import exists
    if !ctrl_content.contains(&method_lower) {
        if let Some(pos) = ctrl_content.find("use rocket::{") {
            let insert_at = pos + "use rocket::{".len();
            ctrl_content.insert_str(insert_at, &format!("{}, ", method_lower));
        }
    }

    // Implement trait method
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

        // Add Rocket route handler function
        let handler_code = format!(
            "\n#[{}(\"{}\")]\nfn {}() -> {}Response<String> {{\n    {}Controller::{}()\n}}\n",
            method_lower, route_path, action, singular_pascal, singular_pascal, action
        );

        if let Some(pos) = ctrl_content.find("pub fn get_routes()") {
            ctrl_content.insert_str(pos, &handler_code);
        }

        // Add route to routes![...]
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
    } else {
        println!("[4/5] Route already exists in controller: {}", controller_file);
    }

    // Ensure controllers/<plural>/mod.rs exists
    let submod_file = format!("{}/mod.rs", controller_dir);
    if !Path::new(&submod_file).exists() {
        let submod_code = format!(
            "#[path = \"{}.controller.rs\"]\npub mod controller;\n\n#[path = \"{}.interface.rs\"]\npub mod interface;\n",
            singular, singular
        );
        fs::write(&submod_file, submod_code).expect("Failed to write controller mod.rs");
    }

    // Register in src/controllers/mod.rs
    let controllers_mod = "src/controllers/mod.rs";
    let mut controllers_mod_content = fs::read_to_string(controllers_mod).unwrap_or_default();
    let mod_decl = format!("pub mod {};\n", plural);
    if !controllers_mod_content.contains(&mod_decl) {
        controllers_mod_content.push_str(&mod_decl);
        fs::write(controllers_mod, controllers_mod_content).expect("Failed to update controllers/mod.rs");
    }

    // -------------------------------------------------------------
    // 5. Mount & PublicRoutes in main.rs
    // -------------------------------------------------------------
    let main_file = "src/main.rs";
    let mut main_content = fs::read_to_string(main_file).unwrap_or_default();

    // If public route, register in PublicRoutes
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

    // Mount controller in main.rs if not mounted
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
        "[5/5] Updated {}: mounted route & configured auth ({})",
        main_file,
        if is_auth { "Protected by AuthFairing" } else { "Registered in PublicRoutes" }
    );

    println!("\nSUCCESS! Endpoint generated completely:");
    println!("  HTTP:   {} /{}{}", method, plural, route_path);
    println!("  Auth:   {}", if is_auth { "Protected (Auth required)" } else { "Open (Public)" });
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
