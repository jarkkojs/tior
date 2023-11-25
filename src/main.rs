// SPDX-License-Identifier: GPL-2.0

mod arguments;
mod session;

use crate::{
    arguments::{Arguments, Command},
    session::Session,
};
use clap::Parser;
use crossterm::{
    event,
    event::{Event, KeyCode, KeyModifiers},
};
use inquire::{
    autocompletion::{Autocomplete, Replacement},
    CustomUserError,
};
use mode::Entry;
use std::io::ErrorKind;

fsmentry::dsl! {
    pub Mode {
        WaitingCommand -> WaitingInput -> WaitingCommand;
    }
}

/// Taken from https://github.com/mikaelmello/inquire/blob/main/inquire/examples/complex_autocompletion.rs
#[derive(Clone, Default)]
pub struct FilePathCompleter {
    input: String,
    paths: Vec<String>,
    lcp: String,
}

impl FilePathCompleter {
    fn update_input(&mut self, input: &str) -> Result<(), CustomUserError> {
        if input == self.input {
            return Ok(());
        }

        self.input = input.to_owned();
        self.paths.clear();

        let input_path = std::path::PathBuf::from(input);

        let fallback_parent = input_path
            .parent()
            .map(|p| {
                if p.to_string_lossy() == "" {
                    std::path::PathBuf::from(".")
                } else {
                    p.to_owned()
                }
            })
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let scan_dir = if input.ends_with('/') {
            input_path
        } else {
            fallback_parent.clone()
        };

        let entries = match std::fs::read_dir(scan_dir) {
            Ok(read_dir) => Ok(read_dir),
            Err(err) if err.kind() == ErrorKind::NotFound => std::fs::read_dir(fallback_parent),
            Err(err) => Err(err),
        }?
        .collect::<Result<Vec<_>, _>>()?;

        let mut idx = 0;
        let limit = 15;

        while idx < entries.len() && self.paths.len() < limit {
            let entry = entries.get(idx).unwrap();

            let path = entry.path();
            let path_str = if path.is_dir() {
                format!("{}/", path.to_string_lossy())
            } else {
                path.to_string_lossy().to_string()
            };

            if path_str.starts_with(&self.input) && path_str.len() != self.input.len() {
                self.paths.push(path_str);
            }

            idx = idx.saturating_add(1);
        }

        self.lcp = self.longest_common_prefix();

        Ok(())
    }

    fn longest_common_prefix(&self) -> String {
        let mut ret: String = String::new();

        let mut sorted = self.paths.clone();
        sorted.sort();
        if sorted.is_empty() {
            return ret;
        }

        let mut first_word = sorted.first().unwrap().chars();
        let mut last_word = sorted.last().unwrap().chars();

        loop {
            match (first_word.next(), last_word.next()) {
                (Some(c1), Some(c2)) if c1 == c2 => {
                    ret.push(c1);
                }
                _ => return ret,
            }
        }
    }
}

impl Autocomplete for FilePathCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        self.update_input(input)?;

        Ok(self.paths.clone())
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<Replacement, CustomUserError> {
        self.update_input(input)?;

        Ok(match highlighted_suggestion {
            Some(suggestion) => Replacement::Some(suggestion),
            None => match self.lcp.is_empty() {
                true => Replacement::None,
                false => Replacement::Some(self.lcp.clone()),
            },
        })
    }
}

fn run_session(args: &Arguments, device: &str) -> std::io::Result<()> {
    let mut session = Session::new(device.to_string(), args)?;
    let mut mode = mode::Mode::new(mode::State::WaitingInput);
    let mut in_buf = [0; 512];
    let mut out_buf = [0; 4];
    loop {
        let size = session.read_port(&mut in_buf)?;
        session.write_output_all(&in_buf[..size])?;
        let event = event::read()?;

        match mode.entry() {
            // Input mode:
            Entry::WaitingInput(it) => {
                match event {
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
                            session.write_port_all(encoded)?;
                        }
                    }
                    Event::Key(ref key)
                        if key.code == KeyCode::Char('t')
                            && key.modifiers == KeyModifiers::CONTROL =>
                    {
                        it.waiting_command();
                    }
                    Event::Resize(columns, rows) => {
                        log::debug!("Resize({}, {})", columns, rows)
                    }
                    event => log::debug!("Unhandled: {:?}", event),
                }
            }
            // Command mode:
            Entry::WaitingCommand(it) => {
                match event {
                    Event::Key(key) => {
                        if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
                            break;
                        }
                    }
                    event => log::debug!("Unhandled: {:?}", event),
                }
                it.waiting_input();
            }
        }
    }
    Ok(())
}

fn run_command(args: Arguments) -> std::io::Result<()> {
    match &args.command {
        Command::Open { device } => run_session(&args, device)?,
        Command::List => {
            let ports = serialport::available_ports()?;
            for port in ports {
                println!("{}", port.port_name);
            }
        }
    }

    Ok(())
}

fn main() {
    pretty_env_logger::init();
    let args = Arguments::parse();
    run_command(args).unwrap_or_else(|e| {
        log::error!("{}", e);
    })
}
