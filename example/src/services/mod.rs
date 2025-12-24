pub mod user;

use self::user::UserService;
use wire::provider;

#[derive(Debug, Clone)]
pub struct App {
    pub user_service: UserService,
}

#[provider]
pub fn provide_app(user_service: &UserService) -> App {
    App {
        user_service: user_service.clone(),
    }
}
