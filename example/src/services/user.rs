use crate::db::DatabasePool;
use std::sync::Arc;
use wire::provider;

#[derive(Debug, Clone)]
pub struct UserService {
    pub pool: Arc<DatabasePool>,
}

#[provider]
pub fn provide_user_service(pool: &Arc<DatabasePool>) -> UserService {
    UserService { pool: pool.clone() }
}
