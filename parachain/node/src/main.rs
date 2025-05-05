//! Substrate Parachain Node Template CLI

#![warn(missing_docs)]

// Пытается взять command из вендора!
// use polkadot_sdk::*;

mod chain_spec;
mod cli;
mod command;
mod fast_command;
mod rpc;
mod service;
mod fast_service;
use std::sync::mpsc;

use log::{debug, error, info, trace, warn};

struct MyLogger {
    sender: mpsc::Sender<String>,
}

impl log::Log for MyLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>)  {
        self.sender.send(record.args().to_string()).unwrap();
    }

    fn flush(&self) {}
}

fn main() -> sc_cli::Result<()> {
	// let (sender, receiver) = mpsc::channel();
    
    // log::set_boxed_logger(Box::new(MyLogger { sender })).unwrap();
    // log::set_max_level(log::LevelFilter::Trace);

	// let fast_thread = std::thread::spawn(move || {
	// fast_command::run()
	// });
	command::run()
}
