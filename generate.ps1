param(
    [string]$TableName
)

$ErrorActionPreference = "Stop"

# Ensure environment has libpq in PATH for diesel execution
$pgBin = "C:\Users\herbitok\.libpq\postgresql-18.6.0-x86_64-pc-windows-msvc\bin"
if ($env:PATH -notlike "*$pgBin*") {
    $env:PATH = "$pgBin;$env:PATH"
}
$dieselCli = "C:\Users\herbitok\.cargo\bin\diesel.exe"

Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host " Rocket + Diesel Clean Architecture Route & CRUD Generator" -ForegroundColor Cyan
Write-Host "=========================================================" -ForegroundColor Cyan

# 1. Prompt for Table Name if not passed
if ([string]::IsNullOrWhiteSpace($TableName)) {
    $TableName = Read-Host "Enter the table name (e.g. products, categories, orders)"
}

$TableName = $TableName.Trim().ToLower()
if ([string]::IsNullOrWhiteSpace($TableName)) {
    Write-Error "Table name cannot be empty."
    exit 1
}

# Determine singular and plural names
$plural = $TableName
if ($plural.EndsWith("ies")) {
    $singular = $plural.Substring(0, $plural.Length - 3) + "y"
} elseif ($plural.EndsWith("es") -and ($plural.EndsWith("shes") -or $plural.EndsWith("ches") -or $plural.EndsWith("xes"))) {
    $singular = $plural.Substring(0, $plural.Length - 2)
} elseif ($plural.EndsWith("s")) {
    $singular = $plural.Substring(0, $plural.Length - 1)
} else {
    $singular = $plural
    $plural = "${singular}s"
}

# PascalCase names
$toTitleCase = {
    param($str)
    $words = $str.Split('_')
    $res = ""
    foreach ($w in $words) {
        if ($w.Length -gt 0) {
            $res += $w.Substring(0, 1).ToUpper() + $w.Substring(1).ToLower()
        }
    }
    return $res
}

$singularPascal = & $toTitleCase $singular
$pluralPascal = & $toTitleCase $plural

Write-Host "`nTarget Entity:" -ForegroundColor Yellow
Write-Host "  Table Name:      $plural"
Write-Host "  Singular Entity: $singular ($singularPascal)"
Write-Host "  Plural Entity:   $plural ($pluralPascal)"

# 2. Check if table already exists in schema.rs or if new migration is needed
$schemaFile = "src/database/schema.rs"
$tableInSchema = $false
if (Test-Path $schemaFile) {
    $content = Get-Content $schemaFile -Raw
    if ($content -match "diesel::table!\s*\{\s*(\w+\s*\([^)]*\)\s*\{[^\}]*\b$plural\b|\s*$plural\s*\()") {
        $tableInSchema = $true
    }
}

if (-not $tableInSchema) {
    Write-Host "`nCreating migration for '$plural' in Neon DB..." -ForegroundColor Green
    
    # Generate migration
    $migOutput = & $dieselCli migration generate "create_$plural" 2>&1
    Write-Host $migOutput

    # Find the newly created migration directory
    $migDir = Get-ChildItem -Path "migrations" -Directory | Where-Object { $_.Name -like "*create_$plural" } | Sort-Object LastWriteTime -Descending | Select-Object -First 1

    if (-not $migDir) {
        Write-Error "Failed to locate generated migration folder."
        exit 1
    }

    $upSql = @"
CREATE TABLE $plural (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR NOT NULL DEFAULT 'system',
    updated_by VARCHAR NOT NULL DEFAULT 'system'
);
"@

    $downSql = "DROP TABLE $plural;`n"

    Set-Content -Path "$($migDir.FullName)\up.sql" -Value $upSql -NoNewline
    Set-Content -Path "$($migDir.FullName)\down.sql" -Value $downSql -NoNewline

    Write-Host "Applying migration to Neon DB via Diesel..." -ForegroundColor Green
    & $dieselCli migration run
    Write-Host "Migration applied successfully and $schemaFile updated!" -ForegroundColor Green
} else {
    Write-Host "`nTable '$plural' already exists in $schemaFile, proceeding with code scaffolding..." -ForegroundColor Cyan
}

# 3. Append Models to src/database/models.rs
$modelsFile = "src/database/models.rs"
$modelCode = @"


// ==========================================
// $singularPascal Models
// ==========================================
use crate::database::schema::$plural;

#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = $plural)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct $singularPascal {
    pub id: uuid::Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_by: String,
    pub updated_by: String,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = $plural)]
pub struct New$singularPascal<'a> {
    pub name: &'a str,
}
"@

$modelsContent = Get-Content $modelsFile -Raw
if ($modelsContent -notmatch "pub struct $singularPascal\b") {
    Add-Content -Path $modelsFile -Value $modelCode
    Write-Host "Updated $modelsFile with $singularPascal & New$singularPascal" -ForegroundColor Green
}

# 4. Generate Usecases (1 file per CRUD action)
$usecasesDir = "src/usecases/$singular"
if (-not (Test-Path $usecasesDir)) {
    New-Item -ItemType Directory -Path $usecasesDir -Force | Out-Null
}

# create_<singular>.rs
$createUsecase = @"
use diesel::prelude::*;
use crate::database::connection::establish_connection;
use crate::database::models::{$singularPascal, New$singularPascal};
use crate::database::schema::$plural;

pub fn execute(name: &str) -> Result<$singularPascal, String> {
    if name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }

    let mut conn = establish_connection();
    let new_record = New$singularPascal { name };

    diesel::insert_into($plural::table)
        .values(&new_record)
        .returning($singularPascal::as_returning())
        .get_result(&mut conn)
        .map_err(|e| e.to_string())
}
"@
Set-Content -Path "$usecasesDir/create_$singular.rs" -Value $createUsecase

# get_<singular>.rs
$getUsecase = @"
use diesel::prelude::*;
use uuid::Uuid;
use crate::database::connection::establish_connection;
use crate::database::models::$singularPascal;
use crate::database::schema::$plural;

pub fn execute(id: Uuid) -> Result<Option<$singularPascal>, String> {
    let mut conn = establish_connection();
    $plural::table
        .find(id)
        .select($singularPascal::as_select())
        .first(&mut conn)
        .optional()
        .map_err(|e| e.to_string())
}
"@
Set-Content -Path "$usecasesDir/get_$singular.rs" -Value $getUsecase

# get_all_<plural>.rs
$getAllUsecase = @"
use diesel::prelude::*;
use crate::database::connection::establish_connection;
use crate::database::models::$singularPascal;
use crate::database::schema::$plural;

pub fn execute() -> Result<Vec<$singularPascal>, String> {
    let mut conn = establish_connection();
    $plural::table
        .select($singularPascal::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}
"@
Set-Content -Path "$usecasesDir/get_all_$plural.rs" -Value $getAllUsecase

# delete_<singular>.rs
$deleteUsecase = @"
use diesel::prelude::*;
use uuid::Uuid;
use crate::database::connection::establish_connection;
use crate::database::schema::$plural;

pub fn execute(id: Uuid) -> Result<usize, String> {
    let mut conn = establish_connection();
    diesel::delete($plural::table.find(id))
        .execute(&mut conn)
        .map_err(|e| e.to_string())
}
"@
Set-Content -Path "$usecasesDir/delete_$singular.rs" -Value $deleteUsecase

# src/usecases/<singular>/mod.rs
$usecaseMod = @"
pub mod create_$singular;
pub mod delete_$singular;
pub mod get_all_$plural;
pub mod get_$singular;
"@
Set-Content -Path "$usecasesDir/mod.rs" -Value $usecaseMod

# Update src/usecases/mod.rs
$rootUsecasesMod = "src/usecases/mod.rs"
$rootUsecaseContent = Get-Content $rootUsecasesMod -Raw
if ($rootUsecaseContent -notmatch "pub mod $singular;") {
    Add-Content -Path $rootUsecasesMod -Value "pub mod $singular;"
}
Write-Host "Created usecases under $usecasesDir/" -ForegroundColor Green

# 5. Generate Service (src/services/<singular>_service.rs)
$serviceFile = "src/services/${singular}_service.rs"
$serviceCode = @"
use uuid::Uuid;
use crate::database::models::$singularPascal;
use crate::usecases::$singular::{create_$singular, delete_$singular, get_all_$plural, get_$singular};

pub struct ${singularPascal}Service;

impl ${singularPascal}Service {
    pub fn create_$singular(name: &str) -> Result<$singularPascal, String> {
        create_${singular}::execute(name)
    }

    pub fn get_${singular}_by_id(id: Uuid) -> Result<Option<$singularPascal>, String> {
        get_${singular}::execute(id)
    }

    pub fn get_all_$plural() -> Result<Vec<$singularPascal>, String> {
        get_all_${plural}::execute()
    }

    pub fn delete_$singular(id: Uuid) -> Result<usize, String> {
        delete_${singular}::execute(id)
    }
}
"@
Set-Content -Path $serviceFile -Value $serviceCode

# Update src/services/mod.rs
$servicesMod = "src/services/mod.rs"
$servicesModContent = Get-Content $servicesMod -Raw
if ($servicesModContent -notmatch "pub mod ${singular}_service;") {
    Add-Content -Path $servicesMod -Value "pub mod ${singular}_service;"
}
Write-Host "Created service: $serviceFile" -ForegroundColor Green

# 6. Generate Interface (src/controllers/<singular>_interface.rs)
$interfaceFile = "src/controllers/${singular}_interface.rs"
$interfaceCode = @"
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Create${singularPascal}Payload {
    pub name: String,
}

pub type ${singularPascal}Response<T> = Result<Json<T>, status::Custom<String>>;

pub fn error_response(status: Status, message: String) -> status::Custom<String> {
    status::Custom(status, message)
}
"@
Set-Content -Path $interfaceFile -Value $interfaceCode

# 7. Generate Controller (src/controllers/<singular>_controller.rs)
$controllerFile = "src/controllers/${singular}_controller.rs"
$controllerCode = @"
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, routes, Route};
use uuid::Uuid;
use crate::controllers::${singular}_interface::{error_response, Create${singularPascal}Payload, ${singularPascal}Response};
use crate::database::models::$singularPascal;
use crate::services::${singular}_service::${singularPascal}Service;

#[get("/")]
fn list_$plural() -> ${singularPascal}Response<Vec<$singularPascal>> {
    ${singularPascal}Service::get_all_$plural()
        .map(Json)
        .map_err(|err| error_response(Status::InternalServerError, err))
}

#[get("/<id>")]
fn get_$singular(id: &str) -> ${singularPascal}Response<$singularPascal> {
    let parsed_id = match Uuid::parse_str(id) {
        Ok(u) => u,
        Err(_) => return Err(error_response(Status::BadRequest, "Invalid UUID format".to_string())),
    };

    match ${singularPascal}Service::get_${singular}_by_id(parsed_id) {
        Ok(Some(item)) => Ok(Json(item)),
        Ok(None) => Err(error_response(Status::NotFound, "$singularPascal not found".to_string())),
        Err(err) => Err(error_response(Status::InternalServerError, err)),
    }
}

#[post("/", data = "<payload>")]
fn create_$singular(payload: Json<Create${singularPascal}Payload>) -> ${singularPascal}Response<$singularPascal> {
    ${singularPascal}Service::create_$singular(&payload.name)
        .map(Json)
        .map_err(|err| error_response(Status::BadRequest, err))
}

#[delete("/<id>")]
fn delete_$singular(id: &str) -> Result<String, rocket::response::status::Custom<String>> {
    let parsed_id = match Uuid::parse_str(id) {
        Ok(u) => u,
        Err(_) => return Err(error_response(Status::BadRequest, "Invalid UUID format".to_string())),
    };

    match ${singularPascal}Service::delete_$singular(parsed_id) {
        Ok(count) => Ok(format!("Deleted {} $singular(s)", count)),
        Err(err) => Err(error_response(Status::InternalServerError, err)),
    }
}

pub fn get_routes() -> Vec<Route> {
    routes![list_$plural, get_$singular, create_$singular, delete_$singular]
}
"@
Set-Content -Path $controllerFile -Value $controllerCode

# Update src/controllers/mod.rs
$controllersMod = "src/controllers/mod.rs"
$controllersModContent = Get-Content $controllersMod -Raw
if ($controllersModContent -notmatch "pub mod ${singular}_controller;") {
    Add-Content -Path $controllersMod -Value "pub mod ${singular}_controller;"
}
if ($controllersModContent -notmatch "pub mod ${singular}_interface;") {
    Add-Content -Path $controllersMod -Value "pub mod ${singular}_interface;"
}
Write-Host "Created controller and interface: $controllerFile" -ForegroundColor Green

# 8. Mount in src/main.rs
$mainFile = "src/main.rs"
$mainContent = Get-Content $mainFile -Raw
$mountLine = "        .mount(`"/$plural`", controllers::${singular}_controller::get_routes())"

if ($mainContent -notmatch "controllers::${singular}_controller::get_routes") {
    $lastIndex = $mainContent.LastIndexOf(".mount(")
    if ($lastIndex -ge 0) {
        $endOfLine = $mainContent.IndexOf("`n", $lastIndex)
        if ($endOfLine -ge 0) {
            $mainContent = $mainContent.Insert($endOfLine + 1, "$mountLine`r`n")
            Set-Content -Path $mainFile -Value $mainContent -NoNewline
            Write-Host "Mounted route '/$plural' in $mainFile" -ForegroundColor Green
        }
    }
}

Write-Host "`nVerifying Rust compilation with 'cargo check'..." -ForegroundColor Cyan
& cargo check

Write-Host "`nSUCCESS! CRUD endpoints for '/$plural' are ready!" -ForegroundColor Green
Write-Host "Endpoints generated:" -ForegroundColor Yellow
Write-Host "  GET    /$plural        -> List all $plural"
Write-Host "  GET    /$plural/<uuid> -> Get $singular by UUID"
Write-Host "  POST   /$plural        -> Create $singular { 'name': '...' }"
Write-Host "  DELETE /$plural/<uuid> -> Delete $singular by UUID"
