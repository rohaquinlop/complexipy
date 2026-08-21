use clap::Parser;

#[derive(Parser)]
#[command(name = "complexipy", version)]
pub struct CliArgs {
    pub paths: Vec<String>,

    #[arg(short, long, value_delimiter = ',')]
    pub exclude: Vec<String>,
}
