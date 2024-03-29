use std::panic;

use aerugo_comp::{backend, Configuration};
use clap::Parser;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod cli;

fn main() {
    let _args = cli::AerugoArgs::parse();
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env()
        .unwrap();
    let subscriber = FmtSubscriber::builder().with_env_filter(env_filter).finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let configuration = Configuration::new(backend::default_backend);
    let executor = configuration.create_server().expect("Failed to create server");

    if let Err(err) = executor.join() {
        panic::resume_unwind(err)
    }
}
