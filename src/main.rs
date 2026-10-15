#[macro_use]
extern crate rocket;

mod controllers;
mod database;
mod services;
mod usecases;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/users", controllers::user_controller::get_routes())
        .mount("/products", controllers::product_controller::get_routes())
}
