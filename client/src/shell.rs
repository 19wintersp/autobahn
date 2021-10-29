use crate::websocket::{ connect, ConnectionSettings, Input, Output };

use std::thread;
use std::io::{ self, ErrorKind, Read, Write };
use std::process::exit;
use std::sync::mpsc::{ self, TryRecvError };
use std::time::Duration;

use vt100::Parser;

const MENU_PREFIX: &str = "\x1b[107;34m Autobahn shell\x1b[30m |";
const MENU_PROMPT: &str = " \x1b[32m^Z for menu \x1b[0m";
const MENU_CMD: &str = " \x1b[32;1m(q)\x1b[22muit, \x1b[1m(Esc)\x1b[22m cancel \x1b[0m";
const MENU_ERROR: &str = " \x1b[31mUnknown command \x1b[0m";
const MENU_CHAR: u8 = 26; // Ctrl+Z
const CLEAR_SCREEN: &str = "\r\x1b[2J\r\x1b[H";
const CLEAR_ROW: &str = "\x1b[2K";
const RESET_CURSOR: &str = "\x1b[H";
const MOVE_CURSOR: &str = "\x1b[%y;%xH";
const END_CURSOR: &str = "\x1b[0m\x1b[?25h";

pub fn start(connection: ConnectionSettings) {
	if let Err(err) = run(connection) {
		let _ = unsafe { crate::console::disable_raw_mode() };

		error!("{}", err);
		exit(1);
	}
}

fn run(connection: ConnectionSettings) -> io::Result<()> {
	let (tx, rx) = connect(connection)?;

	print!("{}", CLEAR_SCREEN);
	let _ = io::stdout().flush();

	unsafe { crate::console::enable_raw_mode() }?;

	let (input_tx, input_rx) = mpsc::channel();

	thread::spawn(move || {
		let mut stdin = io::stdin();
		let mut buffer = [ 0; 256 ];

		loop {
			if let Ok(read) = stdin.read(&mut buffer) {
				let _ = input_tx.send(buffer[..read].to_vec());
			}
		}
	});

	let (mut cols, mut rows) = unsafe { crate::console::term_size() }?;
	let mut parser = Parser::new(rows - 1, cols, 0);
	let _ = tx.send(Input::Winch(cols, rows));

	let _ = show_menu((cols, rows), MENU_PROMPT);

	let mut stdout = io::stdout();
	let mut exit = 0;

	loop {
		let (new_cols, new_rows) = unsafe { crate::console::term_size() }?;
		if new_cols != cols || new_rows != rows {
			cols = new_cols;
			rows = new_rows;

			let _ = tx.send(Input::Winch(cols, rows));

			parser.set_size(rows - 1, cols);
		}

		match rx.try_recv() {
			Ok(Output::Data(data)) => {
				parser.process(data.as_slice());
				
				let contents = parser.screen().contents_formatted();
				let _ = stdout.write(RESET_CURSOR.as_bytes());
				let _ = stdout.write(contents.as_slice());
				let _ = stdout.flush();

				let _ = show_menu((cols, rows), MENU_PROMPT);

				let pos = parser.screen().cursor_position();
				let _ = stdout.write(
					MOVE_CURSOR
						.replace("%x", &(pos.1 + 1).to_string())
						.replace("%y", &(pos.0 + 1).to_string())
						.as_bytes()
				);
				let _ = stdout.flush();
			},
			Ok(Output::Died(code)) => {
				exit = code;
				break
			},
			Err(TryRecvError::Empty) => (),
			_ => return Err(ErrorKind::Other.into()),
		}

		match input_rx.try_recv() {
			Ok(data) => {
				if let Some(p) = data.iter().position(|byte| *byte == MENU_CHAR) {
					if p > 0 {
						let _ = tx.send(Input::Data(data[..p].to_vec()));
					}
					let _ = tx.send(Input::Stop);

					let _ = show_menu((cols, rows), MENU_CMD);

					if let Ok(input) = input_rx.recv() {
						match input[0] as char {
							'\x1b' | 'x' => (),
							'q' => {
								let _ = tx.send(Input::End);
								break
							},
							_ => {
								let _ = show_menu((cols, rows), MENU_ERROR);
								thread::sleep(Duration::from_millis(2000));
							},
						}
					}

					let _ = show_menu((cols, rows), MENU_PROMPT);

					let pos = parser.screen().cursor_position();
					let _ = stdout.write(
						MOVE_CURSOR
							.replace("%x", &(pos.1 + 1).to_string())
							.replace("%y", &(pos.0 + 1).to_string())
							.as_bytes()
					);
					let _ = stdout.flush();

					let _ = tx.send(Input::Continue);
					if p < (data.len() - 1) {
						let _ = tx.send(Input::Data(data[(p+1)..].to_vec()));
					}
				} else {
					let _ = tx.send(Input::Data(data));
				}
			},
			Err(TryRecvError::Empty) => (),
			_ => return Err(ErrorKind::Other.into()),
		}
	}

	unsafe { crate::console::disable_raw_mode() }?;

	print!("{}{}", CLEAR_SCREEN, END_CURSOR);

	if exit == 255 {
		println!("Process died unusually");
	} else {
		println!("Process exited with code {}", exit);
	}

	Ok(())
}

fn show_menu(dim: (u16, u16), message: &str) -> io::Result<()> {
	let mvcs = MOVE_CURSOR
		.replace("%x", "1")
		.replace("%y", &dim.1.to_string());

	let mut stdout = io::stdout();
	stdout.write(mvcs.as_bytes())?;
	stdout.write(CLEAR_ROW.as_bytes())?;
	stdout.write(MENU_PREFIX.as_bytes())?;
	stdout.write(message.as_bytes())?;
	stdout.flush()?;
	
	Ok(())
}
