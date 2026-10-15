use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

/// Request payload for creating a user
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserPayload {
    pub username: String,
    pub email: String,
}

pub type UserResponse<T> = Result<Json<T>, status::Custom<String>>;

pub fn error_response(status: Status, message: String) -> status::Custom<String> {
    status::Custom(status, message)
}
