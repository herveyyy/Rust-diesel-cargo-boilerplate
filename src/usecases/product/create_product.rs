use diesel::prelude::*;
use crate::database::connection::establish_connection;
use crate::database::models::{Product, NewProduct};
use crate::database::schema::products;

pub fn execute(name: &str) -> Result<Product, String> {
    if name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }

    let mut conn = establish_connection();
    let new_record = NewProduct { name };

    diesel::insert_into(products::table)
        .values(&new_record)
        .returning(Product::as_returning())
        .get_result(&mut conn)
        .map_err(|e| e.to_string())
}
