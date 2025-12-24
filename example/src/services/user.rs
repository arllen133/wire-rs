use crate::db::DatabasePool;
use crate::repo::Repository;
use std::sync::Arc;
use wire::provider;

#[derive(Debug, Clone)]
pub struct UserService {
    pub pool: Arc<DatabasePool>,
    pub repo: Arc<dyn Repository>,
}

#[provider]
pub fn provide_user_service(
    pool: &Arc<DatabasePool>, 
    // Injecting a specific implementation directly on the parameter
    #[inject(std::sync::Arc<repo::MockRepository>)]
    repo: &Arc<dyn Repository>,
) -> UserService {
    println!("UserService: Accessing targeted repo -> {}", repo.get_data());
    UserService { 
        pool: pool.clone(),
        repo: repo.clone(),
    }
}
