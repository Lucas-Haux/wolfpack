use clap::Parser;
 
#[derive(Parser)]
#[clap(author, version)]
struct Args {
    #[clap(short, long, value_parser)]
    wolfpack: String,

    #[clap(value_parser)]
    first: f32,

    #[clap(value_parser)]
    second: f32
}

fn main() {
    let args = Args::parse();

    let result = match args.wolfpack.as_str() {
        "+" => args.first + args.second,
        _ => panic!("idk")
    };
    println!("{}", result);
}
