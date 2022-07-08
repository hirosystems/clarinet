use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(short, long, value_parser)]
    tsv_path: String,
}

pub fn main() {
    let args = Args::parse();

    println!("-> {}",  args.tsv_path);
}
