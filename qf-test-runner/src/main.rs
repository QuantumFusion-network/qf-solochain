use clap::Parser;
use tracing_subscriber::prelude::*;

use polkavm::{Config as PolkaVMConfig, Engine, Linker, Module as PolkaVMModule, ProgramBlob};


#[derive(Parser, Debug)]
#[command(name = "QF polkavm blob runner (for now only for `calc`)")]
#[command(version = "1.0")]
#[command(about = "QF polkavm blob runner (for now only for `calc`)", long_about = None)]
struct Cli {
    /// Path to the PolkaVM program to execute
    #[arg(short, long)]
    program: std::path::PathBuf,
    /// Entry point of the program
    #[arg(short, long)]
    entry: String,
    /// List of arguments to pass to the program (for example `-a1 -a2`)
    #[arg(short, long)]
    args: Vec<String>,
}

fn main() {
    let registry = tracing_subscriber::registry();

    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing::Level::INFO.into())
        .from_env_lossy();

    registry
        .with(tracing_subscriber::fmt::layer().with_filter(filter))
        .try_init()
        .expect("Failed to initialize tracing");

    let cli = Cli::parse();

    let raw_blob = std::fs::read(cli.program)
        .map_err(|e| {
            tracing::debug!("Failed to initialize tracing: {}", e);
            e
        })
        .expect("Failed to read program");
    let blob = ProgramBlob::parse(raw_blob.as_slice().into())
        .map_err(|e| {
            tracing::debug!("Failed to parse program blob: {}", e);
            e
        }).expect("Failed to parse program blob");

    let mut config = PolkaVMConfig::from_env()
        .map_err(|e| {
            tracing::debug!("Failed to load config: {}", e);
            e
        }).expect("Failed to load config");
    config.set_allow_dynamic_paging(true);
    let engine = Engine::new(&config)
        .map_err(|e| {
            tracing::debug!("Failed to create engine: {}", e);
            e
        }).expect("Failed to create engine");
    let module = PolkaVMModule::from_blob(&engine, &Default::default(), blob)
        .map_err(|e| {
            tracing::debug!("Failed to create module: {}", e);
            e
        }).expect("Failed to create module");

    let linker: Linker = Linker::new();

    // Link the host functions with the module.
    let instance_pre = linker.instantiate_pre(&module)
        .map_err(|e| {
            tracing::debug!("Failed to link module: {}", e);
            e
        }).expect("Failed to link module");

    // Instantiate the module.
    let mut instance = instance_pre.instantiate()
        .map_err(|e| {
            tracing::debug!("Failed to instantiate module: {}", e);
            e
        }).expect("Failed to instantiate module");

    let args = cli.args.iter().map(|arg| arg.parse::<u32>().unwrap()).collect::<Vec<_>>();
    let res = instance
        .call_typed_and_get_result::<u32, (u32, u32)>(&mut (), cli.entry, (args[0], args[1]))
        .map_err(|e| {
            tracing::debug!("Failed to call function: {:?}", e);
            e
        }).expect("Failed to call function");

    tracing::info!("Result: {:?}", res);
}
