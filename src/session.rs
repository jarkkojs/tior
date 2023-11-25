// SPDX-License-Identifier: GPL-2.0

use clap::{builder::PossibleValuesParser, Parser, Subcommand, ValueEnum};
use core::time::Duration;
use crossterm::{
    execute, terminal,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use serde::Serialize;
use std::io::{self, ErrorKind, Write};

/// Poll rate in Hz
static POLL_RATE: u64 = 100;
/// Poll duration in ms
pub static POLL_DURATION: Duration = Duration::from_millis(1000 / POLL_RATE / 2);

/// Serial port session parity
#[derive(ValueEnum, Clone, Copy, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Parity {
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
enum FlowControl {
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
pub enum Command {
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
    baud_rate: u32,

    /// Line data bits
    #[arg(short, long, default_value_t = String::from("8"), value_parser = PossibleValuesParser::new(["5", "6", "7", "8"]))]
    data_bits: String,

    /// Flow control
    #[arg(short, long, default_value_t, value_enum)]
    flow_control: FlowControl,

    /// Parity
    #[arg(short, long, default_value_t, value_enum)]
    parity: Parity,

    #[command(subcommand)]
    pub command: Command,
}

/// Manages the serial port connection and TTY.
pub struct Session {
    port: Box<dyn serialport::SerialPort>,
}

impl Session {
    /// Create new session.
    pub fn new(device: String, args: &Arguments) -> io::Result<Self> {
        let mut port = serialport::new(device, args.baud_rate)
            .timeout(POLL_DURATION)
            .open()?;

        let data_bits = match args.data_bits.as_str() {
            "5" => serialport::DataBits::Five,
            "6" => serialport::DataBits::Six,
            "7" => serialport::DataBits::Seven,
            "8" => serialport::DataBits::Eight,
            _ => return Err(io::Error::from(ErrorKind::InvalidInput)),
        };

        port.set_data_bits(data_bits)?;
        port.set_stop_bits(serialport::StopBits::One)?;
        port.set_baud_rate(args.baud_rate)?;
        port.set_parity(args.parity.into())?;
        port.set_flow_control(args.flow_control.into())?;

        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Session { port })
    }

    /// Read data from the serial port. Returns `Ok(0)` if the operation expires.
    pub fn read_port(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.port.read(buf).or_else(|e| {
            if e.kind() == ErrorKind::TimedOut {
                Ok(0)
            } else {
                Err(e)
            }
        })
    }

    /// Write data to the serial port.
    pub fn write_port_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.port.write_all(buf)
    }

    /// Write data to the output.
    pub fn write_output_all(&mut self, buf: &[u8]) -> io::Result<()> {
        io::stdout().write_all(buf)?;
        io::stdout().flush()?;
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Disabling RAW mode");
        execute!(io::stdout(), LeaveAlternateScreen).expect("Leaving alternate screen");
    }
}
