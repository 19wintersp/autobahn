mod message;

pub use message::{ Connection, Message };

use std::thread;
use std::io::{ self, Error, ErrorKind };
use std::str::FromStr;
use std::sync::mpsc::{ self, Receiver, Sender };

use websocket::{ ClientBuilder, OwnedMessage };

const PROTOCOL: &str = "autobahn-websocket-tunnel";
const VERSION: (u8, u8) = (0, 2);

// I hate this
pub fn connect(
	options: ConnectionSettings,
) -> io::Result<(Sender<Input>, Receiver<Output>)> {
	let url = format!("wss://{}/__atbws", options.repl.domain());
	let mut client = ClientBuilder::new(url.as_str())
		.map_err(|_| {
			error!("invalid URL {}", url);
			Error::from(ErrorKind::Other)
		})?
		.add_protocol(PROTOCOL)
		.connect_secure(None)
		.map_err(|_| {
			error!("failed to connect websocket");
			Error::from(ErrorKind::Other)
		})?;

	let (input_tx, input_rx) = mpsc::channel();
	let (output_tx, output_rx) = mpsc::channel();

	thread::spawn(move || {
		if client.send_message(
			&OwnedMessage::Binary(
				minicbor::to_vec(
					Message::Hello(VERSION.0, VERSION.1)
				).unwrap()
			)
		).is_err() { return }

		if client.send_message(
			&OwnedMessage::Binary(
				minicbor::to_vec(
					Message::Authenticate(options.key)
				).unwrap()
			)
		).is_err() { return }

		if let Ok(OwnedMessage::Binary(data)) = client.recv_message() {
			if let Ok(message) = minicbor::decode(data.as_slice()) {
				if Message::Authentication(true) != message {
					return
				}
			}
		} else { return }

		if client.send_message(
			&OwnedMessage::Binary(
				minicbor::to_vec(
					Message::ConnectionType(options.connection)
				).unwrap()
			)
		).is_err() { return }
	
		if client.set_nonblocking(true).is_err() { return }

		loop {
			if let Ok(message) = client.recv_message() {
				match message {
					OwnedMessage::Close(_) => break,
					OwnedMessage::Ping(data) => {
						debug!("websocket pinged");
						let _ = client.send_message(&OwnedMessage::Pong(data));
					},
					OwnedMessage::Binary(data) => {
						if let Ok(message) = minicbor::decode(data.as_slice()) {
							match message {
								Message::ChildDeath(exit) => {
									let _ = output_tx.send(Output::Died(exit));
								},
								Message::Error => break,
								Message::SocketClose => {
									let _ = output_tx.send(Output::Closed);
								},
								Message::SocketOutput(data) | Message::TerminalOutput(data) => {
									let _ = output_tx.send(Output::Data(data));
								},
								_ => (),
							}
						}
					},
					_ => (),
				}
			}

			if let Ok(input) = input_rx.try_recv() {
				let _ = client.send_message(
					&OwnedMessage::Binary(
						minicbor::to_vec(
							match input {
								Input::Data(ref data) => match options.connection {
									Connection::Shell => Message::TerminalInput(data.clone()),
									Connection::Port(_) => Message::SocketInput(data.clone()),
								},
								Input::Continue => Message::SignalContinue,
								Input::Stop => Message::SignalStop,
								Input::Winch(w, h) => Message::SignalWinch(w, h),
								Input::End => Message::EndSession,
							}
						).unwrap()
					)
				);

				if input == Input::End {
					break
				}
			}
		}

		if client.shutdown().is_err() {
			warn!("failed to shutdown client");
		}
	});

	Ok((input_tx, output_rx))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Input {
	Data(Vec<u8>),
	Continue,
	Stop,
	Winch(u16, u16),
	End,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Output {
	Data(Vec<u8>),
	Died(u8),
	Closed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionSettings {
	pub repl: Repl,
	pub connection: Connection,
	pub key: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Repl {
	pub name: String,
	pub user: String,
}

impl Repl {
	fn domain(&self) -> String {
		format!("{}.{}.repl.co", self.name, self.user)
	}
}

impl FromStr for Repl {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, ()> {
		let split = s.split('/').collect::<Vec<&str>>();

		Ok(Self {
			user: split[0].strip_prefix('@')
				.unwrap_or(split[0])
				.to_string(),
			name: split.get(1)
				.ok_or(())?
				.to_string(),
		})
	}
}
