
pub trait Repository: Send + Sync + std::fmt::Debug {
    fn get_data(&self) -> String;
}

#[derive(Debug)]
pub struct SqlRepository;

impl Repository for SqlRepository {
    fn get_data(&self) -> String {
        "Data from SQL Database (Primary)".to_string()
    }
}

#[derive(Debug)]
pub struct MockRepository;

impl Repository for MockRepository {
    fn get_data(&self) -> String {
        "Data from Mock Database (Secondary)".to_string()
    }
}

