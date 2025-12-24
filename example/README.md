# Wire-rs Example

This example demonstrates how to use `wire-rs` for dependency injection.

## Project Structure
- `config.rs`: Simple configuration provider.
- `db.rs`: Database pool provider (using `Arc`).
- `services/`: Service layer with nested modules and sets.
- `main.rs`: Entry point with `#[injector(App)]`.

## How to Generate Wire Code
Run the provided generation script:
```bash
bash gen.sh
```
This will run the `wire-bin` tool and generate `src/wire_gen.rs`.

## How to Run
After generating the code, build and run the example:
```bash
cargo run
```

The `main` function in `src/main.rs` will call `init_generated()` from the generate code and print the result.
