mod input;
mod output;

use output::Output;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!(
            "usage: {} <hal_config.yaml> <output-dir>",
            args.get(0).unwrap()
        );
        std::process::exit(1);
    }
    let hal_config_path = args.get(1).unwrap().clone();
    let mut input = input::Input::read(hal_config_path);

    //println!("Input: {}", input.svd);
    //println!("Input: {}", input.hal_definitions);

    let output = Output::from(&mut input);

    //println!("Output: {}", output.hal_definitions);

    let output_dir = args.get(2).unwrap().clone();
    if !output_dir.ends_with("/") {
        println!("output path must end with a /");
        std::process::exit(1);
    }
    output.write(output_dir);
}
