use diesel::prelude::*;
use crate::database::connection::establish_connection;
use crate::database::models::User;
use crate::database::schema::users;

pub fn execute() -> Result<Vec<User>, String> {
    let mut conn = establish_connection();
    users::table
        .select(User::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}
