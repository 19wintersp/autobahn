use crate::websocket::{ connect, ConnectionSettings, Input, Output };

use std::thread;
use std::net::{ Shutdown, TcpListener, TcpStream };
use std::io::{ self, ErrorKind, Read, Write };
use std::process::exit;
use std::sync::mpsc::TryRecvError;

const DEFAULT_PORT: u16 = 3325;

pub fn start(connection: ConnectionSettings, port: Option<u16>) {
	if let Err(err) = run(connection, port.unwrap_or(DEFAULT_PORT)) {
		error!("{}", err);
		exit(1);
	}
}

fn run(connection: ConnectionSettings, port: u16) -> io::Result<()> {
	let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;

	for stream in listener.incoming() {
		if let Ok(mut stream) = stream {
			let connection = connection.clone();
			thread::spawn(move || {
				if let Err(error) = handle_client(connection, &mut stream) {
					warn!("failed to handle incoming stream");

					let _ = stream.write_fmt(format_args!("atb error {}", error));
					let _ = stream.shutdown(Shutdown::Both);
				}
			});
		} else {
			warn!("incoming stream failed to connect");
		}
	}

	Ok(())
}

fn handle_client(
	connection: ConnectionSettings,
	stream: &mut TcpStream,
) -> io::Result<()> {
	let (tx, rx) = connect(connection)?;

	stream.set_nonblocking(true)?;

	loop {
		let mut buffer = [ 0; 256 ];
		match stream.read(&mut buffer) {
			Ok(0) => {
				let _ = tx.send(Input::End);
				break
			},
			Ok(read) => {
				let _ = tx.send(Input::Data(buffer[..read].to_vec()));
			},
			Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
			Err(error) => return Err(error),
		}

		match rx.try_recv() {
			Ok(output) => match output {
				Output::Data(data) => {
					let _ = stream.write(data.as_slice());
				},
				Output::Closed => break,
				_ => (),
			},
			Err(TryRecvError::Empty) => (),
			Err(_) => return Err(ErrorKind::Other.into()),
		}
	}

	stream.shutdown(Shutdown::Both)?;

	Ok(())
}
