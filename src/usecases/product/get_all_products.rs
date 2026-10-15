use diesel::prelude::*;
use crate::database::connection::establish_connection;
use crate::database::models::Product;
use crate::database::schema::products;

pub fn execute() -> Result<Vec<Product>, String> {
    let mut conn = establish_connection();
    products::table
        .select(Product::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}
