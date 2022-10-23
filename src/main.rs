use std::io::Write;
use std::process;
use clap::Parser;
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
}

fn main() {
    let conf = Configuration::parse();

    match gen::document(&mut std::io::stdout(), conf) {
        Err(e) => {
            std::io::stderr().write(e.to_string().as_bytes()).unwrap();
            process::exit(1);
        },
        Ok(_) => process::exit(0),
    }
}
