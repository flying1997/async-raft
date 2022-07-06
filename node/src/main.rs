use rl_logger::info;
use structopt::StructOpt;
use types::config::NodeConfig;
#[derive(Debug, StructOpt)]
struct Args {
    /// Path to Node Config File 'node.yaml'
    #[structopt(short = "c", long)]
    pub config_path: String,
}

fn main() {
    let args = Args::from_args();
    let node_config = NodeConfig::from_dir(&args.config_path);
    info!("Config:{:?}", node_config);
    node::start(&node_config, None);
}
