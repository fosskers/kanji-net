use gumdrop::{Options, ParsingStyle};
use kn_core::Entry;

#[derive(Options)]
struct Args {
    /// Show this help message.
    help: bool,

    /// Show the current version of `kin`.
    version: bool,

    /// Path to the Kanji data file.
    #[options(free, required)]
    data_path: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse_args_or_exit(ParsingStyle::AllOptions);

    if args.version {
        let version = env!("CARGO_PKG_VERSION");
        println!("{}", version);
    }

    Ok(())
}
