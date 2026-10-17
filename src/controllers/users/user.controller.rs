use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, routes, Route};
use super::interface::{
    error_response, CreateUserPayload, IUserControllerInterface, UserResponse,
};
use crate::database::models::User;
use crate::services::user_service::UserService;

pub struct UserController;

impl IUserControllerInterface for UserController {
    fn list_users() -> UserResponse<Vec<User>> {
        UserService::get_all_users()
            .map(Json)
            .map_err(|err| error_response(Status::InternalServerError, err))
    }

    fn get_user(id: i32) -> UserResponse<User> {
        match UserService::get_user_by_id(id) {
            Ok(Some(user)) => Ok(Json(user)),
            Ok(None) => Err(error_response(Status::NotFound, "User not found".to_string())),
            Err(err) => Err(error_response(Status::InternalServerError, err)),
        }
    }

    fn create_user(payload: Json<CreateUserPayload>) -> UserResponse<User> {
        UserService::create_user(&payload.username, &payload.email)
            .map(Json)
            .map_err(|err| error_response(Status::BadRequest, err))
    }

    fn delete_user(id: i32) -> Result<String, rocket::response::status::Custom<String>> {
        UserService::delete_user(id)
            .map(|count| format!("Deleted {} user(s)", count))
            .map_err(|err| error_response(Status::InternalServerError, err))
    }
}

// --- Route Handlers ---
#[get("/")]
fn list_users() -> UserResponse<Vec<User>> {
    UserController::list_users()
}

#[get("/<id>")]
fn get_user(id: i32) -> UserResponse<User> {
    UserController::get_user(id)
}

#[post("/", data = "<payload>")]
fn create_user(payload: Json<CreateUserPayload>) -> UserResponse<User> {
    UserController::create_user(payload)
}

#[delete("/<id>")]
fn delete_user(id: i32) -> Result<String, rocket::response::status::Custom<String>> {
    UserController::delete_user(id)
}

pub fn get_routes() -> Vec<Route> {
    routes![list_users, get_user, create_user, delete_user]
}
