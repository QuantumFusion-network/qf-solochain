use std::io::Write;

use clap::Parser;
use tracing_subscriber::prelude::*;

extern crate alloc;

use polkavm::{
    Config as PolkaVMConfig,
    Engine, GasMeteringKind, InterruptKind, Module, ModuleConfig,
    ProgramBlob, Reg
};
use polkavm_disassembler::{Disassembler, DisassemblyFormat};

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

const NATIVE_PAGE_SIZE: u32 = 4096;

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
    /// Additional debug information
    #[arg(short, long, action )]
    debug: bool,
    /// Write debug to the file
    #[arg(long)]
    debug_file: Option<std::path::PathBuf>,
}

fn disassemble_with_gas(blob: &ProgramBlob, format: DisassemblyFormat) -> Vec<u8> {
    let mut disassembler = Disassembler::new(blob, format).unwrap();
    disassembler.display_gas().unwrap();

    let mut buffer = Vec::with_capacity(1 << 20);
    disassembler.disassemble_into(&mut buffer).unwrap();
    buffer
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
    let page_size = NATIVE_PAGE_SIZE;
    let blob = ProgramBlob::parse(raw_blob.clone().into())
        .map_err(|e| {
            tracing::debug!("Failed to parse a blob: {}", e);
            e
        })
        .expect("Failed to parse a blob");

    let disasm;
    if cli.debug {
        let dblob = ProgramBlob::parse(raw_blob.into())
            .map_err(|e| {
                tracing::debug!("Failed to parse a blob: {}", e);
                e
            })
            .expect("Failed to parse a blob");
        disasm = disassemble_with_gas(&dblob, DisassemblyFormat::Guest);
        let assembly_text = String::from_utf8(disasm).unwrap();

        println!("\n========={}===========\n", assembly_text);

        let exported_fns = blob.exports().map(|e| e.symbol().clone().to_string()).collect::<Vec<_>>();
        let imported_fns = blob.imports().iter().map(|e| e).collect::<Vec<_>>();
        println!("Imported functions:");
        for imp in imported_fns {
            if imp.is_some() {
                println!("- {:?}", String::from_utf8(imp.expect("no import").as_bytes().to_vec()).unwrap());
            }
        }
        println!("\nExported functions:");
        for exp in exported_fns {
            println!("- {:?}", exp);
        }

        if cli.debug_file.is_some() {
            let mut file = std::fs::File::create(cli.debug_file.as_ref().unwrap()).unwrap();
            let data = assembly_text.as_bytes();
            file.write_all(&data).unwrap();
        }
    }
    println!("\nRun the blob");
    let mut module_config = ModuleConfig::new();
    module_config.set_page_size(page_size);
    module_config.set_gas_metering(Some(GasMeteringKind::Sync));
    if cli.debug {
        module_config.set_step_tracing(true);
    }
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

    println!("Entry \"{}\" with args: {:?}", entry, args);

    instance.prepare_call_typed(pc, args);
    let gas_cost = module.calculate_gas_cost_for(pc).expect(&format!("Failed to calculate gas cost for {:?}", pc));
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
                let pc = instance.program_counter().expect("Failed to get program counter");
                println!("Step pc={:?}", pc);
            },
            InterruptKind::Finished => {
                println!("Finished");
                break;
            },
            InterruptKind::Ecalli(num) => {
                let Some(name) = module.imports().get(num) else {
                    panic!("unexpected external call: {num}");
                };

                if name == "get_third_number" {
                    instance.set_reg(Reg::A0, 100);
                } else {
                    panic!("unexpected external call: {name} ({num})")
                }
            },
            InterruptKind::NotEnoughGas => {
                println!("Not enough gas");
                break;
            },
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
