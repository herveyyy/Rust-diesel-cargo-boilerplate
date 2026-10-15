use diesel::prelude::*;
use crate::database::connection::establish_connection;
use crate::database::models::{NewUser, User};
use crate::database::schema::users;

pub fn execute(username: &str, email: &str) -> Result<User, String> {
    if username.trim().is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    if email.trim().is_empty() {
        return Err("Email cannot be empty".to_string());
    }

    let mut conn = establish_connection();
    let new_user = NewUser { username, email };

    diesel::insert_into(users::table)
        .values(&new_user)
        .returning(User::as_returning())
        .get_result(&mut conn)
        .map_err(|e| e.to_string())
}
