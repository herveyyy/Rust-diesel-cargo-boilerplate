use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use crate::database::models::User;

// --- DTOs / Payloads ---
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserPayload {
    pub username: String,
    pub email: String,
}

pub type UserResponse<T> = Result<Json<T>, status::Custom<String>>;

pub fn error_response(status: Status, message: String) -> status::Custom<String> {
    status::Custom(status, message)
}

// --- Controller Interface Contract ---
pub trait IUserControllerInterface {
    fn list_users() -> UserResponse<Vec<User>>;
    fn get_user(id: i32) -> UserResponse<User>;
    fn create_user(payload: Json<CreateUserPayload>) -> UserResponse<User>;
    fn delete_user(id: i32) -> Result<String, status::Custom<String>>;
}
