use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct Root {
    #[arg(default_value = ".")]
    pub root: String,
}
