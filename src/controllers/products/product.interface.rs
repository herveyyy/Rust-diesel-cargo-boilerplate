use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use crate::database::models::Product;

// --- DTOs / Payloads ---
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProductPayload {
    pub name: String,
}

pub type ProductResponse<T> = Result<Json<T>, status::Custom<String>>;

pub fn error_response(status: Status, message: String) -> status::Custom<String> {
    status::Custom(status, message)
}

// --- Controller Interface Contract ---
pub trait IProductControllerInterface {
    fn list_products() -> ProductResponse<Vec<Product>>;
    fn get_product(id: &str) -> ProductResponse<Product>;
    fn create_product(payload: Json<CreateProductPayload>) -> ProductResponse<Product>;
    fn delete_product(id: &str) -> Result<String, status::Custom<String>>;
}
