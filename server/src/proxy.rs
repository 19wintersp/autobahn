use crate::{ PROXY_PORT, SERVER_PORT };

use std::thread;
use std::io::{ self, ErrorKind, Read, Write };
use std::net::{ self, TcpListener, TcpStream };
use std::sync::mpsc::Receiver;

const SERVER_PATH: &str = "/__atbws";

pub fn start(port: Option<u16>, signaler: Receiver<()>) -> io::Result<()> {
	info!("proxy running");

	let address = format!("0.0.0.0:{}", PROXY_PORT);
	let listener = TcpListener::bind(address.as_str())?;

	listener.set_nonblocking(true)?;
	
	for stream in listener.incoming() {
		if let Ok(mut stream) = stream {
			trace!("received stream");

			thread::spawn(move || {
				if handle_stream(&mut stream, port).is_err() {
					warn!("stream handler failed");
					let _ = stream.shutdown(net::Shutdown::Both);
				}
			});
		}

		if signaler.try_recv().is_ok() {
			break
		}
	}

	Ok(())
}

fn handle_stream(stream: &mut TcpStream, mut port: Option<u16>) -> io::Result<()> {
	let mut buffer = [ 0; 256 ];
	let read = stream.read(&mut buffer)?;

	let request_line = &*String::from_utf8_lossy(&buffer[..read]);
	if let Some(path_start) = request_line.find(' ') {
		let path = request_line.get((path_start + 1)..).unwrap();
		if path.starts_with(SERVER_PATH) {
			port = Some(SERVER_PORT);
		}
	}

	if port.is_none() {
		return Err(ErrorKind::AddrNotAvailable.into())
	}

	let dest_address = format!("0.0.0.0:{}", port.unwrap());
	let mut dest = TcpStream::connect(dest_address)?;

	dest.write(&buffer[..read])?;

	stream.set_nonblocking(true)?;
	dest.set_nonblocking(true)?;

	loop {
		match stream.read(&mut buffer) {
			Ok(0) => break,
			Ok(read) => {
				dest.write(&buffer[..read])?;
			},
			Err(error) => match error.kind() {
				ErrorKind::WouldBlock => (),
				_ => return Err(error),
			},
		};

		match dest.read(&mut buffer) {
			Ok(0) => break,
			Ok(read) => {
				stream.write(&buffer[..read])?;
			},
			Err(error) => match error.kind() {
				ErrorKind::WouldBlock => (),
				_ => return Err(error),
			},
		};
	}

	stream.shutdown(net::Shutdown::Both)?;
	dest.shutdown(net::Shutdown::Both)?;

	Ok(())
}
