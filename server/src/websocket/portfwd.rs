use super::{ Input, Output };

use std::thread;
use std::sync::mpsc::{ self, Receiver, Sender };
use std::io::{ self, Read, Write };
use std::net::{ Shutdown, TcpStream };

pub(super) fn handle_client(
	port: u16,
) -> io::Result<(Sender<Input>, Receiver<Output>)> {
	let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))?;

	let (input_tx, input_rx) = mpsc::channel();
	let (output_tx, output_rx) = mpsc::channel();

	stream.set_nonblocking(true)?;

	thread::spawn(move || loop {
		let mut buffer = [ 0; 256 ];
		if let Ok(read) = stream.read(&mut buffer) {
			if read == 0 {
				let _ = output_tx.send(Output::Closed);
				break
			} else {
				let _ = output_tx.send(Output::Data(buffer[..read].to_vec()));
			}
		}

		if let Ok(input) = input_rx.try_recv() {
			match input {
				Input::Data(data) => {
					let _ = stream.write(data.as_slice());
				},
				Input::End => {
					let _ = stream.shutdown(Shutdown::Both);
					break
				},
				_ => unreachable!(),
			}
		}
	});

	Ok((input_tx, output_rx))
}
