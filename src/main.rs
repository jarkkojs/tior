// SPDX-License-Identifier: GPL-2.0-or-later
//! A command-line interface (cli) for the serial port.

mod arguments;
mod path_complete;
mod serial_port;
mod terminal;

use arguments::{Arguments, Task};
use clap::Parser;
use terminal::Terminal;

fn try_main(args: Arguments, terminal: Terminal) -> std::io::Result<()> {
    match &args.task {
        Task::Open { device } => terminal.run(&args, device),
        Task::List => terminal
            .available_ports()
            .map(|ports| ports.into_iter().for_each(|p| println!("{p}"))),
    }
}

fn main() {
    pretty_env_logger::init();

    let terminal = Terminal;
    let args = Arguments::parse();

    try_main(args, terminal).unwrap_or_else(|e| {
        log::error!("{}", e);
    })
}
