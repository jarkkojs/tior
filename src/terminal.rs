// SPDX-License-Identifier: GPL-2.0-or-later
//! A command-line interface (CLI) driven by a finite-state machine

use crate::arguments::Arguments;
use crate::path_complete::PathComplete;
use crate::serial_port::SerialPort;
use mode::{Entry, Mode, ReceivingFile, SendingFile, WaitingCommand, WaitingInput};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Read, Write};

fsmentry::dsl! {
    #[derive(Debug)]
    pub Mode {
        WaitingInput -> WaitingCommand -> WaitingInput;
        WaitingCommand -> SendingFile -> WaitingInput;
        WaitingCommand -> ReceivingFile -> WaitingInput;
        WaitingCommand -> Exit;
    }
}

pub struct Terminal;

impl Terminal {
    pub fn run(&self, args: &Arguments, device: &str) -> io::Result<()> {
        let mut port = SerialPort::new(device.to_string(), args)?;
        let mut mode = Mode::new(mode::State::WaitingInput);
        let mut buf: [u8; 512] = [0; 512];

        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;

        loop {
            let size = port.read(&mut buf)?;

            io::stdout().write_all(&buf[..size])?;
            io::stdout().flush()?;

            match mode.entry() {
                Entry::WaitingInput(it) => self.visit_waiting_input(it, &mut port),
                Entry::WaitingCommand(it) => self.visit_waiting_command(it, &mut port),
                Entry::SendingFile(it) => self.visit_sending_file(it, &mut port),
                Entry::ReceivingFile(it) => self.visit_receiving_file(it, &mut port),
                Entry::Exit => return Ok(()),
            }?;
        }
    }

    pub fn available_ports(&self) -> io::Result<Vec<String>> {
        let ports: Vec<String> = serialport::available_ports()?
            .iter()
            .map(|p| p.port_name.clone())
            .collect();
        Ok(ports)
    }

    fn try_drop(&self) -> io::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)
    }

    fn visit_waiting_input(&self, it: WaitingInput, port: &mut SerialPort) -> io::Result<()> {
        match event::read()? {
            Event::Key(ref key) if key.modifiers == KeyModifiers::NONE => {
                // The buffer is sized to fit any UTF-8 character (max 4 bytes):
                let mut buf: [u8; 4] = [0; 4];

                // TODO: Substitute later on with a hash table with `KeyEvent`
                // as the lookup, thus allowing run-time configuration.
                let encoded = match key.code {
                    // UTF-8:
                    KeyCode::Char(ch) => ch.encode_utf8(&mut buf).as_bytes(),
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
                    port.write_all(encoded)?;
                }
            }
            Event::Key(ref key)
                if key.code == KeyCode::Char('t') && key.modifiers == KeyModifiers::CONTROL =>
            {
                it.waiting_command();
            }
            event => log::trace!("unhandled: {event:?}"),
        }
        Ok(())
    }

    fn visit_waiting_command(&self, it: WaitingCommand, _: &mut SerialPort) -> io::Result<()> {
        match event::read()? {
            Event::Key(ref key) if key.modifiers == KeyModifiers::NONE => {
                // TODO: Substitute later on with a hash table with `KeyEvent`
                // as the lookup, thus allowing run-time configuration.
                match key.code {
                    KeyCode::Char('q') => it.exit(),
                    KeyCode::Char('s') => it.sending_file(),
                    KeyCode::Char('r') => it.receiving_file(),
                    _ => it.waiting_input(),
                }
            }
            event => log::trace!("unhandled: {event:?}"),
        }
        Ok(())
    }

    fn visit_sending_file(&self, it: SendingFile, _: &mut SerialPort) -> io::Result<()> {
        let current_dir = std::env::current_dir()?;
        let help_message = format!("PWD: {}", current_dir.to_string_lossy());
        let path = inquire::Text::new("Send")
            .with_autocomplete(PathComplete::default())
            .with_help_message(&help_message)
            .prompt()
            .unwrap_or_default();

        log::debug!("send: {path}");

        // TODO: zmodem
        it.waiting_input();
        Ok(())
    }

    fn visit_receiving_file(&self, it: ReceivingFile, _: &mut SerialPort) -> io::Result<()> {
        // TODO: zmodem
        it.waiting_input();
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.try_drop().unwrap_or_else(|e| {
            log::error!("drop: {e}");
        })
    }
}
