use clap::Parser;
use tracing_subscriber::prelude::*;

extern crate alloc;

use polkavm::{
    Config as PolkaVMConfig,
    Engine, GasMeteringKind, InterruptKind, Module, ModuleConfig,
    ProgramBlob
};

// For debugging purposes
macro_rules! match_interrupt {
    ($interrupt:expr, $pattern:pat) => {
        let i = $interrupt;
        assert!(
            matches!(i, $pattern),
            "unexpected interrupt: {i:?}, expected: {:?}",
            stringify!($pattern)
        );
    };
}

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Path to the PolkaVM program to execute
    #[arg(short, long)]
    program: std::path::PathBuf,
    /// Entry point (default "add_numbers")
    #[arg(short, long, default_value="add_numbers")]
    entry_name: String,
    /// "a" param (default 1)
    #[arg(short, long, default_value="1")]
    a: u32,
    /// "b" param (default 2)
    #[arg(short, long, default_value="2")]
    b: u32,
    /// Available Gas
    #[arg(short, long, default_value="100")]
    gas: u32,
}

fn get_native_page_size() -> usize {
    4096
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

    println!("Program: {:?}", cli.program);
    let raw_blob = std::fs::read(cli.program).expect("Failed to read program");
    let config = PolkaVMConfig::from_env()
        .map_err(|e| {
            tracing::debug!("Failed to initialize PolkaVM Config: {}", e);
            e
        })
        .expect("Failed to initialize PolkaVM Config");
    let engine = Engine::new(&config)
        .map_err(|e| {
            tracing::debug!("Failed to initialize PolkaVM Engine: {}", e);
            e
        })
        .expect("Failed to initialize PolkaVM Engine");
    let page_size = get_native_page_size() as u32;
    let blob = ProgramBlob::parse(raw_blob.into())
        .map_err(|e| {
            tracing::debug!("Failed to parse a blob: {}", e);
            e
        })
        .expect("Failed to parse a blob");

    let mut module_config = ModuleConfig::new();
    module_config.set_page_size(page_size);
    module_config.set_gas_metering(Some(GasMeteringKind::Sync));
    let module = Module::from_blob(&engine, &module_config, blob)
        .map_err(|e| {
            tracing::debug!("Failed to initialize PolkaVM Module: {}", e);
            e
        })
        .expect("Failed to initialize PolkaVM Module");

    let mut instance = module.instantiate()
        .map_err(|e| {
            tracing::debug!("Failed to initialize PolkaVM Instance: {}", e);
            e
        })
        .expect("Failed to initialize PolkaVM Instance");

    instance.set_gas(cli.gas.into());
    println!("Gas available: {}", instance.gas());

    let entry: &str = cli.entry_name.as_str();

    let entry_point = module.exports().find(|export| export.symbol() == entry).expect("Entry point not found");

    let pc = entry_point.program_counter();

    let args = (cli.a, cli.b);

    println!("Entry {} with args: {:?}", entry, args);

    instance.prepare_call_typed(pc, args);
    let gas_cost = module.calculate_gas_cost_for(pc).unwrap();
    println!("gas_cost: {}", gas_cost);

    loop {
        let run_result = instance.run()
            .map_err(|e| {
                tracing::debug!("Failed to run an Instance: {}", e);
                e
            })
            .expect("Failed to run an Instance");
        match run_result {
            InterruptKind::Step => {
                println!("Step pc={:?}", instance.program_counter().unwrap());
            },
            InterruptKind::Finished => {
                println!("Finished");
                break;
            }
            _ => {
                println!("Unexpected interrupt: {:?}", run_result);
                break;
            },
        }
    }
    let res = instance.get_result_typed::<u32>();
    println!("Gas left: {}", instance.gas());
    println!("Result: {}", res);
}
