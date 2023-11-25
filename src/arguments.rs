// SPDX-License-Identifier: GPL-2.0-or-later
//! Reads and interprets command-line arguments.

use clap::{builder::PossibleValuesParser, Parser, Subcommand, ValueEnum};
use core::time::Duration;
use serde::Serialize;

/// Poll rate in Hz
static POLL_RATE: u64 = 100;
/// Poll duration in ms
pub static POLL_DURATION: Duration = Duration::from_millis(1000 / POLL_RATE / 2);

/// Serial port session parity
#[derive(ValueEnum, Clone, Copy, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Parity {
    #[default]
    None,
    Odd,
    Even,
}

impl From<Parity> for serialport::Parity {
    fn from(val: Parity) -> Self {
        match val {
            Parity::None => serialport::Parity::None,
            Parity::Odd => serialport::Parity::Odd,
            Parity::Even => serialport::Parity::Even,
        }
    }
}

/// Serial port session flow control method
#[derive(clap::ValueEnum, Clone, Copy, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlowControl {
    #[default]
    None,
    Software,
    Hardware,
}

impl From<FlowControl> for serialport::FlowControl {
    fn from(val: FlowControl) -> Self {
        match val {
            FlowControl::None => serialport::FlowControl::None,
            FlowControl::Software => serialport::FlowControl::Software,
            FlowControl::Hardware => serialport::FlowControl::Hardware,
        }
    }
}

/// Serial port session task
#[derive(Subcommand, Debug)]
pub enum Task {
    /// Open Terminal
    Open { device: String },
    /// List available devices
    List,
}

/// Arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(about = "Connect to serial port", long_about = None)]
pub struct Arguments {
    /// Line baud rate
    #[arg(short, long, default_value_t = 115200)]
    pub baud_rate: u32,

    /// Line data bits
    #[arg(short, long, default_value_t = String::from("8"), value_parser = PossibleValuesParser::new(["5", "6", "7", "8"]))]
    pub data_bits: String,

    /// Flow control
    #[arg(short, long, default_value_t, value_enum)]
    pub flow_control: FlowControl,

    /// Parity
    #[arg(short, long, default_value_t, value_enum)]
    pub parity: Parity,

    #[command(subcommand)]
    pub task: Task,
}
