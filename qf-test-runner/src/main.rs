use clap::Parser;
use tracing_subscriber::prelude::*;

use polkavm::{Config as PolkaVMConfig, Engine, Linker, Module as PolkaVMModule, ProgramBlob, State};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Path to the PolkaVM program to execute
    #[arg(short, long)]
    program: std::path::PathBuf,
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

    let cli = Cli::try_parse().expect("Failed to parse CLI arguments");

    let raw_blob = std::fs::read(cli.program).expect("Failed to read program");
    let blob = ProgramBlob::parse(raw_blob.as_slice().into()).unwrap();

    let mut config = PolkaVMConfig::from_env().unwrap();
    config.set_allow_dynamic_paging(true);
    let engine = Engine::new(&config).unwrap();
    let module = PolkaVMModule::from_blob(&engine, &Default::default(), blob).unwrap();

    let linker: Linker = Linker::new();

    // Link the host functions with the module.
    let instance_pre = linker.instantiate_pre(&module).unwrap();

    // Instantiate the module.
    let mut instance = instance_pre.instantiate().unwrap();

    let mut state = State::new(vec![]);

    let res = instance
        .call_typed_and_get_result::<u32, (u32, u32)>(&mut state, "add_numbers", (1, 2))
        .unwrap();

    tracing::info!("Result: {:?}", res);
}
