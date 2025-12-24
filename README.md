# wire-rs

`wire-rs` is a zero-overhead, fully automatic, macro-driven Dependency Injection (DI) framework for Rust. It eliminates the boilerplate of manual dependency wiring through compile-time scanning and macro expansion, providing robust type safety and developer productivity.

## 🌟 Key Features

- **🚀 Automatic Scanning**: `wire-build` automatically discovers all functions marked with `#[provider]` during the build process.
- **🧩 Smart Pointer Adaptation**: Seamlessly handles `Arc<T>`, `Box<T>`, and `Rc<T>`. If a provider returns `Arc<T>` and a consumer requires `&T`, the macro automatically inserts `.as_ref()`.
- **✨ First-class Trait Support**: Full support for `dyn Trait` injection with automatic type coercion from concrete implementations.
- **🎯 Targeted Injection**: Use `#[inject(Type)]` on parameters to precisely override dependencies when multiple implementations of a trait exist.
- **🛡️ First-class Result Support**: Providers can return `Result`. The macro handles `?` error propagation automatically.
- **🔍 Compile-time Validation**: Detects **circular dependencies**, **missing providers**, and **type conflicts** during compilation.
- **📦 Zero Runtime Overhead**: All wiring is expanded at compile-time with no performance penalty at runtime.
- **🎨 Highly Customizable**: Supports custom smart pointer wrappers and multiple configuration files.

## 🛠️ Quick Start

### 1. Configure `build.rs`

Integrate the scanner in your `build.rs`:

```rust
fn main() {
    let src_dir = std::path::PathBuf::from("src");
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let providers_path = out_dir.join("providers.json");

    // Automatically scan src and generate provider metadata
    wire_build::generate(&src_dir, &providers_path).expect("failed to scan providers");

    println!("cargo:rerun-if-changed=src");
}
```

### 2. Define Providers

Mark your builder functions with `#[provider]`:

```rust
use wire::provider;

pub struct Config;
pub struct Database;

#[provider]
pub fn provide_config() -> Config {
    Config
}

#[provider]
pub fn provide_db(cfg: &Config) -> Result<Database, MyError> {
    Ok(Database::connect(cfg)?)
}
```

### 3. Inject with a Macro

Use `#[wire]` on your initialization entry point:

```rust
use wire::wire;

#[wire]
pub fn initialize_app() -> Result<App, MyError> {}

fn main() -> Result<(), MyError> {
    let app = initialize_app()?; // Macro has automatically wired all dependencies and handled Results
    Ok(())
}
```

## 💡 Advanced Usage

### Trait Object & Targeted Injection
`wire-rs` makes it easy to work with abstractions. If you have multiple implementations of a trait, you can use `#[inject]` to specify which one to use for a particular parameter:

```rust
#[provider]
pub fn provide_sql_repo() -> Arc<SqlRepository> { ... }

#[provider]
pub fn provide_mock_repo() -> Arc<MockRepository> { ... }

#[provider]
pub fn provide_user_service(
    // Force this parameter to use the Mock implementation
    #[inject(std::sync::Arc<repo::MockRepository>)]
    repo: &Arc<dyn Repository>
) -> UserService { ... }
```
The macro will automatically handle the coercion from `Arc<MockRepository>` to `&Arc<dyn Repository>`.

### Smart Pointer Adaptation
When a provider returns a wrapped type but a consumer needs a reference to the inner type, `wire-rs` handles it automatically:
```rust
#[provider]
pub fn provide_arc_db() -> Arc<Database> { ... }

#[provider]
pub fn provide_repo(db: &Database) -> Repo { ... } // Macro automatically inserts .as_ref()
```

### Custom Wrappers
If you use custom smart pointers, you can specify them in the macro:
```rust
#[wire(wrappers = ["Arc", "MyBox"])]
```

### Fault Tolerance
The `wire-build` scanner is designed to be robust. If a source file has syntax errors, the scanner logs a warning and skips that file, allowing the standard Rust compiler to provide accurate error messages without crashing the build script.

## 📝 License

This project is licensed under the Apache License 2.0.