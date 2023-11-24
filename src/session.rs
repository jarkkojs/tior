// SPDX-License-Identifier: GPL-2.0

use clap::{builder::PossibleValuesParser, Parser, Subcommand};
use core::time::Duration;
use crossterm::{
    execute, terminal,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use serde::Serialize;
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::io::{self};

/// Poll rate in Hz
static POLL_RATE: u64 = 100;
/// Poll duration in ms
pub static POLL_DURATION: Duration = Duration::from_millis(1000 / POLL_RATE / 2);

/// Argument structure matching `serialport::Parity`
#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ParityArg {
    #[default]
    None,
    Odd,
    Even,
}

impl From<ParityArg> for Parity {
    fn from(val: ParityArg) -> Self {
        match val {
            ParityArg::None => Parity::None,
            ParityArg::Odd => Parity::Odd,
            ParityArg::Even => Parity::Even,
        }
    }
}

/// Argument structure matching `serialport::FlowControl`
#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum FlowControlArg {
    #[default]
    None,
    Software,
    Hardware,
}

impl From<FlowControlArg> for FlowControl {
    fn from(val: FlowControlArg) -> Self {
        match val {
            FlowControlArg::None => FlowControl::None,
            FlowControlArg::Software => FlowControl::Software,
            FlowControlArg::Hardware => FlowControl::Hardware,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Open TTY
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
    baud_rate: u32,

    /// Line data bits
    #[arg(short, long, default_value_t = String::from("8"), value_parser = PossibleValuesParser::new(["5", "6", "7", "8"]))]
    data_bits: String,

    /// Flow control
    #[arg(short, long, default_value_t, value_enum)]
    flow_control: FlowControlArg,

    /// Parity
    #[arg(short, long, default_value_t, value_enum)]
    parity: ParityArg,

    #[command(subcommand)]
    pub command: Commands,
}

pub struct Session {
    pub port: Box<dyn SerialPort>,
}

impl Session {
    pub fn new(device: String, args: Arguments) -> Result<Self, Box<dyn std::error::Error>> {
        let mut port = serialport::new(device, args.baud_rate)
            .timeout(POLL_DURATION)
            .open()?;

        let data_bits = match args.data_bits.as_str() {
            "5" => DataBits::Five,
            "6" => DataBits::Six,
            "7" => DataBits::Seven,
            "8" => DataBits::Eight,
            d => return Err(format!("data-bits: {}", d).into()),
        };

        port.set_data_bits(data_bits)?;
        port.set_stop_bits(StopBits::One)?;
        port.set_baud_rate(args.baud_rate)?;
        port.set_parity(args.parity.into())?;
        port.set_flow_control(args.flow_control.into())?;

        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Session { port })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Disabling RAW mode");
        execute!(io::stdout(), LeaveAlternateScreen).expect("Leaving alternate screen");
    }
}
