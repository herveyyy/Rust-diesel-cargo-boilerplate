use diesel::prelude::*;
use crate::database::connection::establish_connection;
use crate::database::schema::users;

pub fn execute(user_id: i32) -> Result<usize, String> {
    let mut conn = establish_connection();
    diesel::delete(users::table.find(user_id))
        .execute(&mut conn)
        .map_err(|e| e.to_string())
}
