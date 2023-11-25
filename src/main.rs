// SPDX-License-Identifier: GPL-2.0-or-later
//! Provides a command-line interface (CLI) driven by a finite-state machine
//! (FSM).

mod arguments;
mod path_complete;
mod session;

use crate::{
    arguments::{Arguments, Task},
    session::Session,
};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use mode::{Entry, ReceivingFile, SendingFile, WaitingCommand, WaitingInput};
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

/// Visit `WaitingInput` state.
fn visit_waiting_input(it: WaitingInput, session: &mut Session) -> std::io::Result<()> {
    match event::read()? {
        Event::Key(ref key) if key.modifiers == KeyModifiers::NONE => {
            let mut out = [0; 4];
            let encoded = match key.code {
                // UTF-8:
                KeyCode::Char(ch) => ch.encode_utf8(&mut out).as_bytes(),
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
                session.write_all(encoded)?;
            }
        }
        Event::Key(ref key)
            if key.code == KeyCode::Char('t') && key.modifiers == KeyModifiers::CONTROL =>
        {
            it.waiting_command();
        }
        Event::Resize(columns, rows) => {
            log::debug!("columns: {}, rows: {}", columns, rows)
        }
        event => log::debug!("unknown: {:?}", event),
    }

    Ok(())
}

/// Visit `WaitingCommand` state.
fn visit_waiting_command(it: WaitingCommand, _: &mut Session) -> std::io::Result<()> {
    if let Event::Key(key) = event::read()? {
        if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
            it.exit();
        } else if key.code == KeyCode::Char('s') && key.modifiers == KeyModifiers::NONE {
            it.sending_file();
        } else {
            it.waiting_input();
        }
    }

    Ok(())
}

fn visit_sending_file(it: SendingFile, _: &mut Session) -> std::io::Result<()> {
    let current_dir = std::env::current_dir()?;
    let help_message = format!("PWD: {}", current_dir.to_string_lossy());

    let path = inquire::Text::new("Send")
        .with_autocomplete(path_complete::PathComplete::default())
        .with_help_message(&help_message)
        .prompt()
        .unwrap_or_default();

    log::debug!("path: {path}");

    // TODO: ZMODEM send transmission.

    it.waiting_input();
    Ok(())
}

/// TODO: Implement.
fn visit_receiving_file(it: ReceivingFile, _: &mut Session) -> std::io::Result<()> {
    it.waiting_input();
    Ok(())
}

/// Run `Task::Open`.
fn run_open(args: &Arguments, device: &str) -> std::io::Result<()> {
    let mut session = Session::new(device.to_string(), args)?;
    let mut mode = mode::Mode::new(mode::State::WaitingInput);
    let mut buf = [0; 512];

    loop {
        let size = session.read(&mut buf)?;
        io::stdout().write_all(&buf[..size])?;
        io::stdout().flush()?;

        match mode.entry() {
            Entry::WaitingInput(it) => visit_waiting_input(it, &mut session),
            Entry::WaitingCommand(it) => visit_waiting_command(it, &mut session),
            Entry::SendingFile(it) => visit_sending_file(it, &mut session),
            Entry::ReceivingFile(it) => visit_receiving_file(it, &mut session),
            Entry::Exit => return Ok(()),
        }?;

        log::trace!("mode: {:?}", mode.state());
    }
}

/// Run `Task::List`.
fn run_list() -> std::io::Result<()> {
    let ports = serialport::available_ports()?;

    for port in ports {
        println!("{}", port.port_name);
    }

    Ok(())
}

fn run_task(args: Arguments) -> std::io::Result<()> {
    match &args.task {
        Task::Open { device } => run_open(&args, device)?,
        Task::List => run_list()?,
    }

    Ok(())
}

fn main() {
    pretty_env_logger::init();
    let args = Arguments::parse();
    run_task(args).unwrap_or_else(|e| {
        log::error!("{}", e);
    })
}
