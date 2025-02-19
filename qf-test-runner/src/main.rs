use clap::Parser;
use tracing_subscriber::prelude::*;

use polkavm::{Config as PolkaVMConfig, Engine, Linker, Module as PolkaVMModule, ProgramBlob, State};
use polkavm::Caller;
use polkavm::InterruptKind;
use polkavm::Reg;

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

    let mut linker: Linker = Linker::new();

    linker.define_typed("foo", |caller: Caller| -> u32 { caller.user_data.foo }).unwrap();
    linker.define_typed("transfer", |caller: Caller| -> u32 { caller.user_data.address[2].into() }).unwrap();

    // Link the host functions with the module.
    let instance_pre = linker.instantiate_pre(&module).unwrap();

    // Instantiate the module.
    let mut instance = instance_pre.instantiate().unwrap();

    let mut state = State::new(
         vec![1,2,3],
         100,
    );

    // linker.define_typed("foo", |caller: Caller| -> u32 { 42 }).unwrap();
    // linker.define_typed("foo", |caller: Caller| -> u32 { 42 }).unwrap();
    // linker.define_typed("transfer", |caller: Caller| -> u32 { caller.user_data.address[2].into() }).unwrap();

    let res = instance
        .call_typed_and_get_result::<u32, (u32, u32)>(&mut state, "add_numbers", (1, 2));
    
    tracing::info!("res: {:?}", res);

    tracing::info!("Result: {:?}", res.unwrap());
    
    let entry_point = module.exports().find(|export| export == "add_numbers").unwrap().program_counter();
    let mut instance = module.instantiate().unwrap();
    instance.set_next_program_counter(entry_point);
    instance.set_reg(Reg::A0, 1);
    instance.set_reg(Reg::A1, 10);
    instance.set_reg(Reg::RA, polkavm::RETURN_TO_HOST);
    instance.set_reg(Reg::SP, module.default_sp());

    println!("Calling into the guest program (low level):");
    loop {
        let interrupt_kind = instance.run().unwrap();
        match interrupt_kind {
            InterruptKind::Finished => break,
            InterruptKind::Ecalli(num) => {
                let Some(name) = module.imports().get(num) else {
                    panic!("unexpected external call: {num}");
                };

                println!("name: {}", name);

                if name == "foo" {
                    instance.set_reg(Reg::A0, 1000);
                } else if name == "transfer" {
                    instance.set_reg(Reg::A0, 3);
                } else {
                    panic!("unexpected external call: {name} ({num})")
                }
            }
            _ => panic!("unexpected interruption: {interrupt_kind:?}"),
        }
    }

    println!("  1 + 10 + 1000 + 3 = {}", instance.reg(Reg::A0));
}
