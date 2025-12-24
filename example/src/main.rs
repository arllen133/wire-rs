mod config;
mod db;
mod services;

use wire::wire;
use std::error::Error;

// The `wire` macro will generate the body of this function.
// It will now handle Result-returning providers automatically.
#[wire]
pub fn initialize_app() -> Result<services::App, Box<dyn Error>> {}

fn main() -> Result<(), Box<dyn Error>> {
    let app = initialize_app()?;
    println!("Successfully initialized App!");
    println!("User Service Pool: {:?}", app.user_service.pool);
    Ok(())
}
