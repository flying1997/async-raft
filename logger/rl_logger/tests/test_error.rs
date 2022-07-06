use std::path::PathBuf;
use std::sync::Arc;
use rl_logger::{debug, error, info, Rlogger, Level};
use rl_logger::prelude::FileWriter;

fn _setup_logger_env(log_file: Option<PathBuf>)-> Option<Arc<Rlogger>> {
    let mut logger = rl_logger::Logger::new();
    logger
        .channel_size(10000)
        .is_async(true)
        .level(Level::Trace)
        .read_env();
    if let Some(log_file) = log_file {
        logger.printer(Box::new(FileWriter::new(log_file)));
    }
    let logger = Some(logger.build());
    return logger;
}
#[test]
fn test_logger() {
    let _test_logger = _setup_logger_env(None);
    let round = 0;
    info!("Current round = {}",round);
    debug!("debug");
    error!("error");
    info!("info");
}