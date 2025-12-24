use wire::provider;

pub struct Config;

#[provider]
pub fn provide_config() -> Config {
    Config
}
