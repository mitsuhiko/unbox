mod archive;
mod cli;
mod utils;
mod zip;

fn main() {
    crate::cli::main().unwrap();
}
