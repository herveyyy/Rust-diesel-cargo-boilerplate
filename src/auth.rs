use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::uri::Origin;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::{routes, Data, Route};
use std::collections::HashSet;

use crate::database::models::User;
use crate::services::user_service::UserService;

use rocket::http::Method;

/// Registry for explicitly public routes (publicRoute = true)
/// All routes are protected by default unless added here.
/// Supports both:
/// - Exact method: "GET /products/*" or .open_get("/products/*")
/// - All methods:  "/health" or .open("/health")
#[derive(Clone, Default)]
pub struct PublicRoutes {
    exact: HashSet<(Option<Method>, &'static str)>,
    prefixes: Vec<(Option<Method>, &'static str)>,
}

#[allow(dead_code)]
impl PublicRoutes {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Mark a route or wildcard prefix as public.
    /// Can include HTTP method (e.g. "GET /products/*" or "/health")
    pub fn open(mut self, rule: &'static str) -> Self {
        let (method, path) = Self::parse_rule(rule);
        self.add(method, path);
        self
    }

    /// Mark route as public ONLY for GET requests
    pub fn open_get(self, path: &'static str) -> Self {
        self.open_method(Method::Get, path)
    }

    /// Mark route as public ONLY for POST requests
    pub fn open_post(self, path: &'static str) -> Self {
        self.open_method(Method::Post, path)
    }

    /// Mark route as public for a specific HTTP method
    pub fn open_method(mut self, method: Method, path: &'static str) -> Self {
        self.add(Some(method), path);
        self
    }

    /// Mark multiple routes as public at once
    pub fn open_all(mut self, rules: &[&'static str]) -> Self {
        for &rule in rules {
            self = self.open(rule);
        }
        self
    }

    fn add(&mut self, method: Option<Method>, path: &'static str) {
        if let Some(prefix) = path.strip_suffix("/*") {
            self.prefixes.push((method, prefix));
        } else {
            self.exact.insert((method, path));
        }
    }

    fn parse_rule(rule: &'static str) -> (Option<Method>, &'static str) {
        let parts: Vec<&str> = rule.split_whitespace().collect();
        if parts.len() == 2 {
            let method = match parts[0].to_uppercase().as_str() {
                "GET" => Some(Method::Get),
                "POST" => Some(Method::Post),
                "PUT" => Some(Method::Put),
                "DELETE" => Some(Method::Delete),
                "PATCH" => Some(Method::Patch),
                _ => None,
            };
            (method, parts[1])
        } else {
            (None, rule.trim())
        }
    }

    pub fn is_public(&self, method: Method, path: &str) -> bool {
        // 1. Check exact match (either matching method or wildcard method None)
        if self.exact.contains(&(Some(method), path)) || self.exact.contains(&(None, path)) {
            return true;
        }

        // 2. Check prefix match
        self.prefixes.iter().any(|(m, prefix)| {
            (m.is_none() || *m == Some(method))
                && (path == *prefix || path.starts_with(&format!("{}/", prefix)))
        })
    }
}

impl<const N: usize> From<[&'static str; N]> for PublicRoutes {
    fn from(arr: [&'static str; N]) -> Self {
        Self::empty().open_all(&arr)
    }
}

/// Request guard that resolves the authenticated PostgreSQL User via Diesel
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AuthUser(pub User);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // 1. Private cookie (AES-GCM encrypted by Rocket secret key)
        let id_str = req
            .cookies()
            .get_private("user_id")
            .map(|c| c.value().to_string())
            // 2. Or Authorization: Bearer <user_id>
            .or_else(|| {
                req.headers()
                    .get_one("Authorization")
                    .and_then(|h| h.strip_prefix("Bearer "))
                    .map(|t| t.trim().to_string())
            });

        if let Some(raw_id) = id_str {
            if let Ok(user_id) = raw_id.parse::<i32>() {
                if let Ok(Some(user)) = UserService::get_user_by_id(user_id) {
                    return Outcome::Success(AuthUser(user));
                }
            }
        }

        Outcome::Forward(Status::Unauthorized)
    }
}

/// Native Rocket session helpers
#[allow(dead_code)]
pub fn login(cookies: &CookieJar<'_>, user_id: i32) {
    cookies.add_private(Cookie::new("user_id", user_id.to_string()));
}

#[allow(dead_code)]
pub fn logout(cookies: &CookieJar<'_>) {
    cookies.remove_private("user_id");
}

/// Fairing enforcing default-deny: all routes require auth unless in PublicRoutes
pub struct AuthFairing;

#[rocket::async_trait]
impl Fairing for AuthFairing {
    fn info(&self) -> Info {
        Info {
            name: "Default-Deny Auth Guard",
            kind: Kind::Request,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _: &mut Data<'_>) {
        let path = req.uri().path().as_str();

        // Internal unauthorized handler is exempt
        if path == "/_auth_unauthorized" {
            return;
        }

        // Check if route is toggled as public for this HTTP method
        let is_public = req
            .rocket()
            .state::<PublicRoutes>()
            .map(|pr| pr.is_public(req.method(), path))
            .unwrap_or(false);

        if is_public {
            return;
        }

        // Validate presence of credentials
        let has_cookie = req.cookies().get_private("user_id").is_some();
        let has_token = req
            .headers()
            .get_one("Authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false);

        if !has_cookie && !has_token {
            req.set_uri(Origin::parse("/_auth_unauthorized").unwrap());
        }
    }
}

// Internal unauthorized endpoint handlers
#[rocket::get("/_auth_unauthorized")]
fn unauth_get() -> (Status, &'static str) {
    (Status::Unauthorized, "{\"error\":\"Unauthorized: Authentication required\"}")
}

#[rocket::post("/_auth_unauthorized")]
fn unauth_post() -> (Status, &'static str) {
    (Status::Unauthorized, "{\"error\":\"Unauthorized: Authentication required\"}")
}

#[rocket::put("/_auth_unauthorized")]
fn unauth_put() -> (Status, &'static str) {
    (Status::Unauthorized, "{\"error\":\"Unauthorized: Authentication required\"}")
}

#[rocket::delete("/_auth_unauthorized")]
fn unauth_delete() -> (Status, &'static str) {
    (Status::Unauthorized, "{\"error\":\"Unauthorized: Authentication required\"}")
}

#[rocket::patch("/_auth_unauthorized")]
fn unauth_patch() -> (Status, &'static str) {
    (Status::Unauthorized, "{\"error\":\"Unauthorized: Authentication required\"}")
}

pub fn get_auth_routes() -> Vec<Route> {
    routes![unauth_get, unauth_post, unauth_put, unauth_delete, unauth_patch]
}
