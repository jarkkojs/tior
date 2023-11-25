// SPDX-License-Identifier: GPL-2.0

use crate::arguments::{Arguments, POLL_DURATION};
use crossterm::{
    execute, terminal,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, ErrorKind, Write};

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
