use diesel::prelude::*;
use crate::database::connection::establish_connection;
use crate::database::models::User;
use crate::database::schema::users;

pub fn execute(user_id: i32) -> Result<Option<User>, String> {
    let mut conn = establish_connection();
    users::table
        .find(user_id)
        .select(User::as_select())
        .first(&mut conn)
        .optional()
        .map_err(|e| e.to_string())
}
