# wire-rs Example

This project demonstrates the core capabilities of `wire-rs`, including automatic provider scanning, smart pointer adaptation, and fallible initialization support.

## Key Features Demonstrated

1.  **Automatic Build-time Scanning**: The `build.rs` script uses `wire-build` to scan the source code for providers and generate metadata used by the macros.
2.  **Trait Binding (`#[bind]`)**: Demonstrates how to map a specific implementation (e.g., `SqlRepository`) to an abstract interface (`dyn Repository`) without boilerplate.
3.  **NewType Pattern**: Shows how to use unique types (e.g., `MockRepo`) to inject a specific implementation when multiple versions of the same Trait exist.
4.  **Fallible Providers**: The `provide_pool` in `src/db.rs` returns a `Result`, demonstrating how `wire-rs` handles error propagation.
5.  **Smart Pointer Adaptation**: The database pool is provided as an `Arc<DatabasePool>`, but consumed as a reference or cloned. `wire-rs` handles this conversion automatically.

## Project Structure

-   `src/db.rs`: Defines a fallible database provider returning `Arc<DatabasePool>`.
-   `src/config.rs`: A simple configuration provider.
-   `src/services/`: contains the business logic.
    -   `user.rs`: `UserService` which depends on the `DatabasePool`.
    -   `mod.rs`: The `App` struct which aggregates services.
-   `src/main.rs`: The entry point that uses `#[wire]` to bootstrap the application.
-   `build.rs`: The build script that triggers the provider scanning.

## How to Run

Since `wire-rs` is fully integrated into the standard Cargo build process, you can simply run:

```bash
cargo run
```

### What happens behind the scenes?
1.  **Cargo** runs `build.rs`.
2.  `build.rs` calls `wire-build` to scan `src/` for functions with `#[provider]`.
3.  `wire-build` generates a `providers.json` (and a cache file) in the `OUT_DIR`.
4.  When compiling `src/main.rs`, the `#[wire]` procedural macro reads `providers.json`.
5.  The macro resolves the dependency graph and generates the code to instantiate all components in the correct order.
6.  The final binary is compiled and executed.

## Testing Fault Tolerance

You can try introducing a syntax error into any `.rs` file. You will notice that:
1.  The `wire-build` scanner will log a warning about the file and skip it.
2.  Standard Rust compiler errors will be displayed clearly.
3.  Once fixed, saving the file will automatically trigger a re-scan and a successful build.
