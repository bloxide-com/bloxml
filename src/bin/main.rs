use bloxml::actor::Actor;
use bloxml::create;
use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the XML file
    #[arg(value_name = "XML_FILE")]
    xml_file: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let actor = Actor::from_xml_file(&args.xml_file)?;
    create::create_module(&actor)
}
