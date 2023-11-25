// SPDX-License-Identifier: GPL-2.0-or-later
//! A serial port interface.

use crate::arguments::{Arguments, POLL_DURATION};
use std::io::{self, ErrorKind};

/// A serial port connector.
pub struct SerialPort(Box<dyn serialport::SerialPort>);

impl SerialPort {
    /// Connect to a serial port.
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

        Ok(Self(port))
    }
}

impl io::Read for SerialPort {
    /// Read data from the serial port. Returns zero length for the buffer,
    /// if the operation expires.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf).or_else(|e| {
            if e.kind() == ErrorKind::TimedOut {
                Ok(0)
            } else {
                Err(e)
            }
        })
    }
}

impl io::Write for SerialPort {
    /// Write data to the serial port. Returns zero length for the buffer,
    /// if the operation expires.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf).or_else(|e| {
            if e.kind() == ErrorKind::TimedOut {
                Ok(0)
            } else {
                Err(e)
            }
        })
    }

    // Flush the intermediate buffer.
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
