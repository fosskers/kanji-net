use gumdrop::Options;
use kn_core::Entry;

#[derive(Options)]
struct Args {
    /// Show this help message.
    help: bool,

    /// Path to the Kanji data file.
    #[options(free, required)]
    data_path: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse_args_default_or_exit();

    Ok(())
}
