use clap::builder::PossibleValuesParser;
use clap::{Parser, Subcommand};
use core::time::Duration;
use crossterm::{event, execute, terminal};
use crossterm::{
    event::{Event, KeyCode, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use serde::Serialize;
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::io::{self, Write};

/// Poll rate in Hz
static POLL_RATE: u64 = 100;
/// Poll duration in ms
static POLL_DURATION: Duration = Duration::from_millis(1000 / POLL_RATE / 2);

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

/// Arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(about = "Connect to serial port", long_about = None)]
struct Arguments {
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
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Open TTY
    Open { device: String },
    /// List available devices
    List,
}

struct Session {
    port: Box<dyn SerialPort>,
}

impl Session {
    fn new(device: String, args: Arguments) -> Result<Self, Box<dyn std::error::Error>> {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    let args = Arguments::parse();

    match &args.command {
        Commands::Open { device } => {
            let mut session = Session::new(device.to_string(), args)?;
            let mut in_buf = [0; 512];
            let mut out_buf = [0; 4];
            let mut prefix = false;
            let mut quit = false;

            while !quit {
                if event::poll(POLL_DURATION)? {
                    match event::read()? {
                        Event::Key(ref key)
                            if key.code == KeyCode::Char('t')
                                && key.modifiers == KeyModifiers::CONTROL
                                && !prefix =>
                        {
                            prefix = true;
                        }
                        Event::Key(key) if prefix => {
                            if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE
                            {
                                quit = true;
                            }
                            prefix = false;
                        }
                        Event::Key(ref key) if key.modifiers == KeyModifiers::NONE => {
                            let encoded = match key.code {
                                // UTF-8:
                                KeyCode::Char(ch) => ch.encode_utf8(&mut out_buf).as_bytes(),
                                KeyCode::Backspace => &[8],
                                KeyCode::Tab => &[9],
                                KeyCode::Enter => &[10],
                                KeyCode::Esc => &[27],
                                // Escape:
                                KeyCode::Up => &[27, 91, 65],
                                KeyCode::Down => &[27, 91, 66],
                                KeyCode::Right => &[27, 91, 67],
                                KeyCode::Left => &[27, 91, 68],
                                KeyCode::End => &[27, 91, 70],
                                KeyCode::Home => &[27, 91, 72],
                                KeyCode::BackTab => &[27, 91, 90],
                                KeyCode::Insert => &[27, 91, 50, 126],
                                KeyCode::Delete => &[27, 91, 51, 126],
                                KeyCode::PageUp => &[27, 91, 53, 126],
                                KeyCode::PageDown => &[27, 91, 54, 126],
                                _ => &[],
                            };

                            if !encoded.is_empty() {
                                match session.port.write(encoded) {
                                    Ok(_) => (),
                                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                                    Err(_) => quit = true,
                                }
                            }
                        }
                        Event::Resize(columns, rows) => {
                            log::debug!("Resize({}, {})", columns, rows)
                        }
                        event => log::debug!("Unhandled: {:?}", event),
                    }
                }

                // Due to error, while writing to the serial port:
                if quit {
                    log::debug!("Quit");
                    break;
                }

                match session.port.read(&mut in_buf) {
                    Ok(size) => {
                        io::stdout().write_all(&in_buf[..size])?;
                        io::stdout().flush()?;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(_) => quit = true,
                }
            }
        }
        Commands::List => {
            let ports = serialport::available_ports()?;
            for port in ports {
                println!("{}", port.port_name);
            }
        }
    }
    Ok(())
}
