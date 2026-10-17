---
trigger: always_on
---

If you ever want to add an endpoint manually (e.g., `POST /users/change-password`):

1. **Usecase** (`src/usecases/user/change_password.rs`):
   - Write the Diesel query in a standalone `execute(user_id, new_password)` function.
   - Register in `src/usecases/user/mod.rs`.
2. **Service** (`src/services/user_service.rs`):
   - Add a forwarding method `change_password` that calls `change_password::execute(...)`.
3. **Interface** (`src/controllers/user_interface.rs`):
   - Add the request payload: `ChangePasswordPayload { new_password: String }`.
4. **Controller** (`src/controllers/user_controller.rs`):
   - Add the route:
     ```rust
     #[post("/<id>/change-password", data = "<payload>")]
     fn change_password(id: i32, payload: Json<ChangePasswordPayload>) -> UserResponse<String> {
         UserService::change_password(id, &payload.new_password)
             .map(|msg| Json(msg))
             .map_err(|err| error_response(Status::BadRequest, err))
     }
     ```
   - Add the function name to `routes![...]` at the bottom of the controller.
5. **Test**: Run `cargo check` and `cargo run`.
