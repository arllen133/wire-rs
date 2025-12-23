pub mod generator;
pub mod graph;
pub mod locator;
pub mod parser;

use std::path::PathBuf;

pub fn exec_wire(root: PathBuf, entry_file: PathBuf) {
    match generate_code(root, entry_file, "App", "init_app") {
        // Default for legacy exec_wire
        Ok(code) => println!("{}", code),
        Err(e) => eprintln!("Error: {}", e),
    }
}

pub fn generate_code(
    crate_root: PathBuf,
    entry_file: PathBuf,
    target_type: &str,
    injector_fn: &str,
) -> Result<String, String> {
    let mut scanner = parser::Scanner::new(crate_root);
    let providers = scanner.run(entry_file, target_type, injector_fn);

    // 3. Build Graph & Solve
    let sorted = graph::DependencyGraph::solve(providers)?;

    // 4. Generate Code
    let code = generator::generate(sorted, injector_fn, target_type);

    Ok(code)
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[test]
    fn test_integration_flow() {
        // 1. Setup mock file system
        let root = std::env::current_dir().unwrap().join("test_integration");
        let src = root.join("src");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&src).unwrap();

        // 2. Define components: Config, Database, Service, Controller

        // src/config.rs
        fs::write(
            src.join("config.rs"),
            r#"
            pub struct Config;
            pub fn provide_config() -> Config { Config }
        "#,
        )
        .unwrap();

        // src/db.rs -> needs Config
        fs::write(
            src.join("db.rs"),
            r#"
            use crate::config::Config;
            pub struct Database;
            pub fn provide_database(_cfg: &Config) -> Database { Database }
        "#,
        )
        .unwrap();

        // src/service.rs -> needs Database
        fs::write(
            src.join("service.rs"),
            r#"
            use crate::db::Database;
            pub struct Service;
            pub fn provide_service(_db: &Database) -> Service { Service }
        "#,
        )
        .unwrap();

        // src/app.rs -> needs Service (The Root)
        fs::write(
            src.join("app.rs"),
            r#"
            use crate::service::Service;
            pub struct App { _service: Service }
            pub fn init_app(service: &Service) -> App {
                App { _service: Service } // dummy copy
            }
        "#,
        )
        .unwrap();

        // src/di.rs -> The Injector Entry
        fs::write(
            src.join("di.rs"),
            r#"
            use crate::config::provide_config;
            use crate::db::provide_database;
            use crate::service::provide_service;
            use crate::app::{init_app, App};

            #[injector(init_app)]
            pub fn di_config() {
                let _ = (provide_config, provide_database, provide_service, init_app);
            }
        "#,
        )
        .unwrap();

        // 3. Run Generator
        let code = super::generate_code(root.clone(), src.join("di.rs"), "App", "init_app")
            .expect("Generation failed");

        // 4. Verify Content
        println!("{}", code);

        let config_idx = code
            .find("provide_config (")
            .expect("Missing provide_config call");
        let db_idx = code
            .find("provide_database (")
            .expect("Missing provide_database call");
        let service_idx = code
            .find("provide_service (")
            .expect("Missing provide_service call");
        let app_idx = code.find("init_app (").expect("Missing init_app call");

        assert!(config_idx < db_idx, "Config should be before Database");
        assert!(db_idx < service_idx, "Database should be before Service");
        assert!(service_idx < app_idx, "Service should be before App");

        assert!(code.contains("let config = crate::config::provide_config ();"));
        assert!(code.contains("let database = crate::db::provide_database (&config);"));
        assert!(code.contains("let service = crate::service::provide_service (&database);"));
        // 注意：generator 目前生成的变量名是基于 output type 的 lowercase。
        // provide_service 返回 Service -> service
        // init_app (App) -> app

        // Clean up
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_cycle_detection() {
        // 1. Setup mock file system
        let root = std::env::current_dir().unwrap().join("test_cycle");
        let src = root.join("src");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&src).unwrap();

        // ServiceA depends on ServiceB
        fs::write(
            src.join("a.rs"),
            r#"
            use crate::b::ServiceB;
            pub struct ServiceA;
            pub fn provide_a(_b: &ServiceB) -> ServiceA { ServiceA }
        "#,
        )
        .unwrap();

        // ServiceB depends on ServiceA
        fs::write(
            src.join("b.rs"),
            r#"
            use crate::a::ServiceA;
            pub struct ServiceB;
            pub fn provide_b(_a: &ServiceA) -> ServiceB { ServiceB }
        "#,
        )
        .unwrap();

        // App depends on ServiceA (Root)
        fs::write(
            src.join("app.rs"),
            r#"
            use crate::a::ServiceA;
            pub struct App;
            pub fn provide_app(_a: &ServiceA) -> App { App }
        "#,
        )
        .unwrap();

        // Entry
        fs::write(
            src.join("di.rs"),
            r#"
            use crate::a::provide_a;
            use crate::b::provide_b;
            use crate::app::{provide_app, App};

            #[injector(init)]
            pub fn di_config() {
                let _ = (provide_a, provide_b, provide_app);
            }
        "#,
        )
        .unwrap();

        // 3. Run Generator
        let result = super::generate_code(root.clone(), src.join("di.rs"), "App", "di_config");

        // 4. Assert Error
        assert!(result.is_err());
        let err = result.err().unwrap();
        println!("Cycle Error: {}", err);
        assert!(err.contains("Cycle detected"));
        // TODO: Expect more details like "ServiceA -> ServiceB -> ServiceA"

        // Clean up
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_name_collision() {
        // 1. Setup mock file system
        let root = std::env::current_dir().unwrap().join("test_collision");
        let src = root.join("src");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&src).unwrap();

        // mod a defines Foo
        fs::write(
            src.join("a.rs"),
            r#"
            pub struct Foo; // A::Foo
            pub fn provide_foo_a() -> Foo { Foo }
        "#,
        )
        .unwrap();

        // mod b defines Foo
        fs::write(
            src.join("b.rs"),
            r#"
            pub struct Foo; // B::Foo
            pub fn provide_foo_b() -> Foo { Foo }
        "#,
        )
        .unwrap();

        // App depends on BOTH
        // init_app(a: a::Foo, b: b::Foo) -> App
        fs::write(
            src.join("app.rs"),
            r#"
            use crate::a::Foo as FooA;
            use crate::b::Foo as FooB;
            pub struct App;
            pub fn init_app(_a: &FooA, _b: &FooB) -> App { App }
        "#,
        )
        .unwrap();

        // Entry
        fs::write(
            src.join("di.rs"),
            r#"
            use crate::a::provide_foo_a;
            use crate::b::provide_foo_b;
            use crate::app::{init_app, App};

            #[injector(init)]
            pub fn di_config() {
                let _ = (provide_foo_a, provide_foo_b, init_app);
            }
        "#,
        )
        .unwrap();

        // 3. Run Generator
        // We expect this to fail or be confused because both providers return "Foo"
        // And the map stores by "Foo".
        let result = super::generate_code(root.clone(), src.join("di.rs"), "App", "di_config");

        // Currently, it probably overwrites the map entry, so one "Foo" wins.
        // And when resolving inputs for init_app: "&FooA" -> "Foo", "&FooB" -> "Foo".
        // Both will resolve to the SAME provider (whichever won).
        // This is WRONG.

        // For now, let's just see what happens.
        if let Ok(code) = result {
            println!("Collision Code:\n{}", code);
        } else {
            println!("Collision Error: {}", result.err().unwrap());
        }

        // Clean up
        let _ = fs::remove_dir_all(&root);
    }
}
