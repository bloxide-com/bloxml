use bloxml::actor::Actor;
use bloxml::create;
use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the JSON file
    #[arg(value_name = "JSON_FILE", short, long)]
    json_file: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let actor = Actor::from_json_file(&args.json_file)?;
    create::create_module(actor)
}
