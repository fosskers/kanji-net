use gumdrop::Options;
use kn_core::Entry;
use rusqlite::{params, Connection};

#[derive(Options)]
struct Args {
    /// Show this help message.
    help: bool,

    /// Path to the SQLite database.
    #[options(free, required)]
    db_path: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse_args_default_or_exit();
    let c = Connection::open(args.db_path)?;

    Ok(())
}
