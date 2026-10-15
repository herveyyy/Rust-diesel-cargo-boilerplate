use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, routes, Route};
use crate::controllers::user_interface::{error_response, CreateUserPayload, UserResponse};
use crate::database::models::User;
use crate::services::user_service::UserService;

#[get("/")]
fn list_users() -> UserResponse<Vec<User>> {
    UserService::get_all_users()
        .map(Json)
        .map_err(|err| error_response(Status::InternalServerError, err))
}

#[get("/<id>")]
fn get_user(id: i32) -> UserResponse<User> {
    match UserService::get_user_by_id(id) {
        Ok(Some(user)) => Ok(Json(user)),
        Ok(None) => Err(error_response(Status::NotFound, "User not found".to_string())),
        Err(err) => Err(error_response(Status::InternalServerError, err)),
    }
}

#[post("/", data = "<payload>")]
fn create_user(payload: Json<CreateUserPayload>) -> UserResponse<User> {
    UserService::create_user(&payload.username, &payload.email)
        .map(Json)
        .map_err(|err| error_response(Status::BadRequest, err))
}

#[delete("/<id>")]
fn delete_user(id: i32) -> Result<String, rocket::response::status::Custom<String>> {
    UserService::delete_user(id)
        .map(|count| format!("Deleted {} user(s)", count))
        .map_err(|err| error_response(Status::InternalServerError, err))
}

pub fn get_routes() -> Vec<Route> {
    routes![list_users, get_user, create_user, delete_user]
}
