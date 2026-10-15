use diesel::prelude::*;
use uuid::Uuid;
use crate::database::connection::establish_connection;
use crate::database::schema::products;

pub fn execute(id: Uuid) -> Result<usize, String> {
    let mut conn = establish_connection();
    diesel::delete(products::table.find(id))
        .execute(&mut conn)
        .map_err(|e| e.to_string())
}
