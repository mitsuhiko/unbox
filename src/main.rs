mod archive;
mod cli;
mod utils;
mod zip;

fn main() {
    use std::io::Write;

    if let Err(err) = crate::cli::main() {
        let mut stderr = std::io::stderr();
        writeln!(&mut stderr, "error: {}", err).ok();
        for cause in err.iter_causes() {
            writeln!(&mut stderr, "  caused by: {}", cause).ok();
        }

        if std::env::var("RUST_BACKTRACE").is_ok() {
            writeln!(&mut stderr, "\n\nerror details:\n{:#?}", err).ok();
        }

        std::process::exit(1);
    }
}
