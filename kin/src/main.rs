use gumdrop::{Options, ParsingStyle};
use kn_core::{Entry, Error};

#[derive(Options)]
struct Args {
    /// Show this help message.
    help: bool,

    /// Show the current version of `kin`.
    version: bool,

    /// Path to the Kanji data file.
    #[options(meta = "PATH", default = "data.json")]
    data: String,
}

fn main() -> Result<(), Error> {
    let args = Args::parse_args_or_exit(ParsingStyle::AllOptions);

    if args.version {
        let version = env!("CARGO_PKG_VERSION");
        println!("{}", version);
    }

    Ok(())
}
