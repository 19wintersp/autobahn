use crate::SERVER_PORT;

mod message;
mod shell;
mod portfwd;

use message::{ Connection, Message };

use std::thread;
use std::io::{ self, Error, ErrorKind };
use std::sync::mpsc::Receiver;

use websocket::OwnedMessage;
use websocket::sync::{ stream, Client, Server };

const PROTOCOL: &str = "autobahn-websocket-tunnel";
const VERSION: (u8, u8) = (0, 2);

pub fn start(key: &str, signaler: Receiver<()>) -> io::Result<()> {
	info!("server running");

	let address = format!("0.0.0.0:{}", SERVER_PORT);
	let mut server = Server::bind(address.as_str())?;

	server.set_nonblocking(true)?;
	
	loop {
		if let Ok(request) = server.accept() {
			trace!("request received");

			let key = String::from(key);
			thread::spawn(move || {
				let mut client = match request.use_protocol(PROTOCOL).accept() {
					Ok(client) => client,
					_ => return,
				};

				if handle_client(&mut client, &key).is_err() {
					warn!("client handler failed");
					let _ = client.shutdown();
				} else {
					trace!("new connection finished");
				}
			});
		}

		if signaler.try_recv().is_ok() { break }
	}

	Ok(())
}

fn handle_client(
	client: &mut Client<stream::TcpStream>,
	key: &str,
) -> io::Result<()> {
	client.set_nonblocking(true)?;

	let mut state = ConnectionState::AwaitingHandshake;
	let mut io = None;

	loop {
		if let Ok(message) = client.recv_message() {
			match message {
				OwnedMessage::Close(_) => {
					/* if state != ConnectionState::SessionEnded {
						warn!("connection closed without wave");

						if let Some((input, _)) = io {
							input.send(Input::End);
						}
					} */

					break
				},
				OwnedMessage::Ping(data) => {
					debug!("websocket pinged");
					client.send_message(&OwnedMessage::Pong(data))
						.map_err(|_| Error::from(ErrorKind::Other))?;
				},
				OwnedMessage::Binary(data) => {
					if let Ok(message) = minicbor::decode(data.as_slice()) {
						match message {
							Message::Authenticate(password) => {
								if state == ConnectionState::AwaitingAuthentication {
									let success = password == key;

									client.send_message(
										&OwnedMessage::Binary(
											minicbor::to_vec(Message::Authentication(success))
												.unwrap()
										)
									).map_err(|_| Error::from(ErrorKind::Other))?;

									if success {
										state = ConnectionState::AwaitingConnection;
									}
								}
							},
							Message::ConnectionType(connection) => {
								if state == ConnectionState::AwaitingConnection {
									if let Connection::Port(port) = connection {
										if let Ok(handler_io) = portfwd::handle_client(port) {
											io = Some(handler_io);
											state = ConnectionState::SocketActive;
										} else {
											let _ = client.send_message(
												&OwnedMessage::Binary(
													minicbor::to_vec(Message::Error).unwrap()
												)
											);
										}
									} else {
										if let Ok(handler_io) = shell::handle_client() {
											io = Some(handler_io);
											state = ConnectionState::ShellActive;
										} else {
											client.send_message(
												&OwnedMessage::Binary(
													minicbor::to_vec(Message::Error).unwrap()
												)
											).map_err(|_| Error::from(ErrorKind::Other))?;
										}
									}
								}
							},
							Message::EndSession => {
								if
									state == ConnectionState::ShellActive ||
									state == ConnectionState::SocketActive
								{
									let _ = io.unwrap().0.send(Input::End);
								}
								
								break
							},
							Message::Hello(vmj, vmn) => {
								if state == ConnectionState::AwaitingHandshake {
									if (vmj, vmn) != VERSION {
										client.send_message(
											&OwnedMessage::Binary(
												minicbor::to_vec(Message::Error).unwrap()
											)
										).map_err(|_| Error::from(ErrorKind::Other))?;

										break
									}

									state = ConnectionState::AwaitingAuthentication;
								}
							},
							Message::SignalContinue => {
								if state == ConnectionState::ShellActive {
									let _ = io.as_ref().unwrap().0.send(Input::Continue);
								}
							},
							Message::SignalStop => {
								if state == ConnectionState::ShellActive {
									let _ = io.as_ref().unwrap().0.send(Input::Stop);
								}
							},
							Message::SignalWinch(w, h) => {
								if state == ConnectionState::ShellActive {
									let _ = io.as_ref().unwrap().0.send(Input::Winch(w, h));
								}
							},
							Message::SocketInput(data) => {
								if state == ConnectionState::SocketActive {
									let _ = io.as_ref().unwrap().0.send(Input::Data(data));
								}
							},
							Message::TerminalInput(data) => {
								if state == ConnectionState::ShellActive {
									let _ = io.as_ref().unwrap().0.send(Input::Data(data));
								}
							},
							_ => (),
						}
					}
				},
				_ => (),
			}
		}

		if let Some((_, ref output)) = io {
			if let Ok(data) = output.try_recv() {
				client.send_message(
					&OwnedMessage::Binary(
						minicbor::to_vec(match data {
							Output::Data(ref data) => match state {
								ConnectionState::ShellActive =>
									Message::TerminalOutput(data.clone()),
								ConnectionState::SocketActive =>
									Message::SocketOutput(data.clone()),
								_ => continue,
							},
							Output::Died(exit) => match state {
								ConnectionState::ShellActive =>
									Message::ChildDeath(exit),
								_ => continue,
							},
							Output::Closed => match state {
								ConnectionState::SocketActive =>
									Message::SocketClose,
								_ => continue,
							},
						}).unwrap()
					)
				).map_err(|_| Error::from(ErrorKind::Other))?;

				if let Output::Died(_) | Output::Closed = data {
					break
				}
			}
		}
	}

	client.shutdown()
}

#[derive(Clone, Debug, PartialEq)]
enum Input {
	Data(Vec<u8>),
	Continue,
	Stop,
	Winch(u16, u16),
	End,
}

#[derive(Clone, Debug, PartialEq)]
enum Output {
	Data(Vec<u8>),
	Died(u8),
	Closed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ConnectionState {
	AwaitingHandshake,
	AwaitingAuthentication,
	AwaitingConnection,
	ShellActive,
	SocketActive,
}
