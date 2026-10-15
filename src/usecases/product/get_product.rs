use diesel::prelude::*;
use uuid::Uuid;
use crate::database::connection::establish_connection;
use crate::database::models::Product;
use crate::database::schema::products;

pub fn execute(id: Uuid) -> Result<Option<Product>, String> {
    let mut conn = establish_connection();
    products::table
        .find(id)
        .select(Product::as_select())
        .first(&mut conn)
        .optional()
        .map_err(|e| e.to_string())
}
