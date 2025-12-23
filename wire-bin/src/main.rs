use clap::Parser;
use std::path::PathBuf;
use wire_cli::generate_code;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "cargo-wire")]
#[command(bin_name = "cargo-wire")] 
enum CargoWire {
   Wire(Args),
}

#[derive(clap::Args, Debug)]
struct Args {
    /// Path to the folder containing the scanner crate root (default: current dir)
    #[arg(short, long, default_value = ".")]
    root: PathBuf,

    /// Path to the entry file relative to root (e.g. src/di.rs)
    #[arg(short, long, default_value = "src/di.rs")]
    file: PathBuf,

    /// Name of the target struct to build (e.g. App)
    #[arg(short = 't', long, default_value = "App")]
    target: String,

    /// Name of the function to generate (e.g. init_app)
    #[arg(short = 'n', long, default_value = "init_app")]
    name: String,
}

fn main() {
    // Check if called as "cargo wire" or just "cargo-wire"
    // Cargo passes the subcommand name as the second argument if called as "cargo wire"
    let args = CargoWire::parse();

    match args {
        CargoWire::Wire(args) => {
             println!("Running wire-rs with root: {:?}, file: {:?}, target: {}, fn: {}", args.root, args.file, args.target, args.name);
             
             let root_path = std::env::current_dir().unwrap().join(args.root);
             let entry_path = root_path.join(args.file);

             match generate_code(root_path, entry_path, &args.target, &args.name) {
                 Ok(code) => {
                     println!("Successfully generated code:");
                     println!("{}", code);
                     // In the future, write to file or stdout based on flags
                 }
                 Err(e) => {
                     eprintln!("Error: {}", e);
                     std::process::exit(1);
                 }
             }
        }
    }
}
