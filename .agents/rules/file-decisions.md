---
trigger: always_on
---

## 🛡️ The Golden Rules to Remember

| Do | Don't |
|---|---|
| ✅ Keep each usecase in its own individual `.rs` file. | ❌ Don't dump multiple usecases into one giant file. |
| ✅ Call **Services** from your Controllers. | ❌ Never import or call Usecases directly inside Controllers. |
| ✅ Call raw Diesel queries inside **Usecases**. | ❌ Don't write SQL or Diesel queries inside Controllers or Services. |
| ✅ Define request/response DTOs inside `<name>_interface.rs`. | ❌ Don't pollute the controller with raw structs. |