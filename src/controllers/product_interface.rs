use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProductPayload {
    pub name: String,
}

pub type ProductResponse<T> = Result<Json<T>, status::Custom<String>>;

pub fn error_response(status: Status, message: String) -> status::Custom<String> {
    status::Custom(status, message)
}
