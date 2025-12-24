mod config;
mod db;
mod repo;
mod services;

use wire::{provider, wire};
use std::error::Error;
use std::sync::Arc;
use crate::repo::Repository;

// The `wire` macro will generate the body of this function.
// It will now handle Result-returning providers automatically.
#[wire]
pub fn initialize_app() -> Result<services::App, Box<dyn Error>> {}

fn main() -> Result<(), Box<dyn Error>> {
    let app = initialize_app()?;
    println!("Successfully initialized App!");
    println!("User Service Pool: {:?}", app.user_service.pool);
    // This should print "Data from Mock Database (Secondary)" because of the override
    println!("User Service Repo Data: {}", app.user_service.repo.get_data());
    Ok(())
}

#[provider]
pub fn provide_repository() -> std::sync::Arc<repo::SqlRepository> {
    std::sync::Arc::new(repo::SqlRepository)
}

#[provider]
pub fn provide_mock_repository() -> std::sync::Arc<repo::MockRepository> {
    std::sync::Arc::new(repo::MockRepository)
}
