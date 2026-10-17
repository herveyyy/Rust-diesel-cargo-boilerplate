#[macro_use]
extern crate rocket;

mod auth;
mod controllers;
mod database;
mod services;
mod usecases;

use auth::{AuthFairing, PublicRoutes};

#[launch]
fn rocket() -> _ {
    let public_routes = PublicRoutes::empty();

    rocket::build()
        .manage(public_routes)
        .attach(AuthFairing)
        .mount("/", auth::get_auth_routes())
        .mount("/users", controllers::users::controller::get_routes())
        .mount("/products", controllers::products::controller::get_routes())
}
