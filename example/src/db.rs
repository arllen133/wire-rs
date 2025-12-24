use crate::config::Config;
use std::sync::Arc;
use wire::provider;

#[derive(Debug)]
pub struct DatabasePool;

#[provider]
pub fn provide_pool(_cfg: &Config) -> Result<Arc<DatabasePool>, Box<dyn std::error::Error>> {
    Ok(Arc::new(DatabasePool))
}
