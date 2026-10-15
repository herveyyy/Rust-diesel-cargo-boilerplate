use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, routes, Route};
use uuid::Uuid;
use crate::controllers::product_interface::{error_response, CreateProductPayload, ProductResponse};
use crate::database::models::Product;
use crate::services::product_service::ProductService;

#[get("/")]
fn list_products() -> ProductResponse<Vec<Product>> {
    ProductService::get_all_products()
        .map(Json)
        .map_err(|err| error_response(Status::InternalServerError, err))
}

#[get("/<id>")]
fn get_product(id: &str) -> ProductResponse<Product> {
    let parsed_id = match Uuid::parse_str(id) {
        Ok(u) => u,
        Err(_) => return Err(error_response(Status::BadRequest, "Invalid UUID format".to_string())),
    };

    match ProductService::get_product_by_id(parsed_id) {
        Ok(Some(item)) => Ok(Json(item)),
        Ok(None) => Err(error_response(Status::NotFound, "Product not found".to_string())),
        Err(err) => Err(error_response(Status::InternalServerError, err)),
    }
}

#[post("/", data = "<payload>")]
fn create_product(payload: Json<CreateProductPayload>) -> ProductResponse<Product> {
    ProductService::create_product(&payload.name)
        .map(Json)
        .map_err(|err| error_response(Status::BadRequest, err))
}

#[delete("/<id>")]
fn delete_product(id: &str) -> Result<String, rocket::response::status::Custom<String>> {
    let parsed_id = match Uuid::parse_str(id) {
        Ok(u) => u,
        Err(_) => return Err(error_response(Status::BadRequest, "Invalid UUID format".to_string())),
    };

    match ProductService::delete_product(parsed_id) {
        Ok(count) => Ok(format!("Deleted {} product(s)", count)),
        Err(err) => Err(error_response(Status::InternalServerError, err)),
    }
}

pub fn get_routes() -> Vec<Route> {
    routes![list_products, get_product, create_product, delete_product]
}
