use std::path::Path;

use borsh::{self, BorshSerialize};
use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
enum SubCommand {
    Encode { file: String },
}

#[derive(Parser, Debug)]
#[clap(version)]
struct Arguments {
    #[command(subcommand)]
    cmd: SubCommand,
}

fn main() {
    let args = Arguments::parse();

    match args.cmd {
        SubCommand::Encode { file } => {
            let path = Path::new(&file);
            let output_path = path.with_extension("borsh");
            println!("output: {}", output_path.display());
            let wasm = std::fs::read(&file).unwrap();
            let output: Vec<u8> = wasm.try_to_vec().unwrap();
            let len = u32::from_le_bytes(output[0..4].try_into().unwrap());
            assert_eq!(len, wasm.len().try_into().unwrap());
            std::fs::write(output_path, output.clone()).unwrap();
            assert_eq!(wasm, output[4..].to_vec());
        }
    }
}
