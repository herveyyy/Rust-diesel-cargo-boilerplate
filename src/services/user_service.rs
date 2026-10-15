use crate::database::models::User;
use crate::usecases::user::{create_user, delete_user, get_all_users, get_user};

pub struct UserService;

impl UserService {
    pub fn create_user(username: &str, email: &str) -> Result<User, String> {
        create_user::execute(username, email)
    }

    pub fn get_user_by_id(id: i32) -> Result<Option<User>, String> {
        get_user::execute(id)
    }

    pub fn get_all_users() -> Result<Vec<User>, String> {
        get_all_users::execute()
    }

    pub fn delete_user(id: i32) -> Result<usize, String> {
        delete_user::execute(id)
    }
}
