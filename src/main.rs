use std::io::{BufWriter, Write};
use std::process;
use clap::Parser;
use rand::{RngCore, SeedableRng, thread_rng};

mod gen;

#[derive(Parser,Default,Debug)]
#[clap(author="Hannah Kolbeck", version, about="A KDL Document Generator")]
pub struct Configuration {
    #[clap(default_value_t=3, short, long)]
    pub depth_max: u32,

    #[clap(default_value_t=10, short, long)]
    pub nodes_per_child_max: u32,

    #[clap(default_value_t=3, short, long)]
    pub extra_space_max: u32,

    #[clap(default_value_t=10, short, long)]
    pub props_or_args_max: u32,

    #[clap(default_value_t=1, short, long)]
    pub blank_lines_max: u32,

    #[clap(default_value_t=20, short, long)]
    pub identifier_len_max: u32,

    #[clap(default_value_t=100, short, long)]
    pub string_len_max: u32,

    #[clap(default_value_t=10, short='l', long)]
    pub num_len_max: u32,

    #[clap(default_value_t=100, short, long)]
    pub comment_len_max: u32,

    #[clap(default_value_t=false, long)]
    pub debug: bool,

    #[clap(long="seed")]
    pub rand_seed: Option<u64>,
}

fn main() {
    let conf = Configuration::parse();
    let mut out = BufWriter::new(std::io::stdout());

    let seed = match conf.rand_seed {
        Some(seed) => seed,
        None => thread_rng().next_u64(),
    };

    std::io::stderr().write(format!("seed: {}\n", seed).as_bytes()).unwrap();
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);

    match gen::document(&mut out, &mut rng, conf) {
        Err(e) => {
            std::io::stderr().write(e.to_string().as_bytes()).unwrap();
            process::exit(1);
        },
        Ok(_) => process::exit(0),
    }
}
