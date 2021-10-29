use std::convert::TryInto;

use minicbor::decode::{ Decode, Decoder, Error as DecodeError };
use minicbor::encode::{ Encode, Encoder, Error as EncodeError };
use minicbor::encode::write::Write;

use num_enum::{ IntoPrimitive, TryFromPrimitive };

#[derive(Clone, Copy, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum MessageType {
	Authenticate,
	Authentication,
	ChildDeath,
	ConnectionType,
	EndSession,
	Error,
	Hello,
	SignalContinue,
	SignalStop,
	SignalWinch,
	SocketClose,
	SocketInput,
	SocketOutput,
	TerminalInput,
	TerminalOutput,
}

impl<'b> Decode<'b> for MessageType {
	fn decode(d: &mut Decoder<'b>) -> Result<Self, DecodeError> {
		let index = d.u8()?;
		index.try_into()
			.map_err(|_| DecodeError::UnknownVariant(index.into()))
	}
}

impl Encode for MessageType {
	fn encode<W: Write>(
		&self,
		e: &mut Encoder<W>,
	) -> Result<(), EncodeError<W::Error>> {
		e.u8((*self).into()).map(|_| ())
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Connection {
	Shell,
	Port(u16),
}

impl From<u16> for Connection {
	fn from(from: u16) -> Self {
		if from == 0 { Self::Shell }
		else { Self::Port(from) }
	}
}

impl Into<u16> for Connection {
	fn into(self) -> u16 {
		if let Self::Port(port) = self { port }
		else { 0 }
	}
}

impl<'b> Decode<'b> for Connection {
	fn decode(d: &mut Decoder<'b>) -> Result<Self, DecodeError> {
		Ok(d.u16()?.into())
	}
}

impl Encode for Connection {
	fn encode<W: Write>(
		&self,
		e: &mut Encoder<W>,
	) -> Result<(), EncodeError<W::Error>> {
		e.u16((*self).into()).map(|_| ())
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
	Authenticate(String),
	Authentication(bool),
	ChildDeath(u8),
	ConnectionType(Connection),
	EndSession,
	Error,
	Hello(u8, u8),
	SignalContinue,
	SignalStop,
	SignalWinch(u16, u16),
	SocketClose,
	SocketInput(Vec<u8>),
	SocketOutput(Vec<u8>),
	TerminalInput(Vec<u8>),
	TerminalOutput(Vec<u8>),
}

impl Message {
	pub fn message_type(&self) -> MessageType {
		match self {
			Self::Authenticate(_) => MessageType::Authenticate,
			Self::Authentication(_) => MessageType::Authentication,
			Self::ChildDeath(_) => MessageType::ChildDeath,
			Self::ConnectionType(_) => MessageType::ConnectionType,
			Self::EndSession => MessageType::EndSession,
			Self::Error => MessageType::Error,
			Self::Hello(_, _) => MessageType::Hello,
			Self::SignalContinue => MessageType::SignalContinue,
			Self::SignalStop => MessageType::SignalStop,
			Self::SignalWinch(_, _) => MessageType::SignalWinch,
			Self::SocketClose => MessageType::SocketClose,
			Self::SocketInput(_) => MessageType::SocketInput,
			Self::SocketOutput(_) => MessageType::SocketOutput,
			Self::TerminalInput(_) => MessageType::TerminalInput,
			Self::TerminalOutput(_) => MessageType::TerminalOutput,
		}
	}
}

impl<'b> Decode<'b> for Message {
	fn decode(d: &mut Decoder<'b>) -> Result<Self, DecodeError> {
		use MessageType::*;

		Ok(match d.decode::<MessageType>()? {
			Authenticate => Self::Authenticate(d.str()?.into()),
			Authentication => Self::Authentication(d.bool()?),
			ChildDeath => Self::ChildDeath(d.u8()?),
			ConnectionType => Self::ConnectionType(d.decode()?),
			EndSession => Self::EndSession,
			Error => Self::Error,
			Hello => Self::Hello(d.u8()?, d.u8()?),
			SignalContinue => Self::SignalContinue,
			SignalStop => Self::SignalStop,
			SignalWinch => Self::SignalWinch(d.u16()?, d.u16()?),
			SocketClose => Self::SocketClose,
			SocketInput => Self::SocketInput(d.bytes()?.into()),
			SocketOutput => Self::SocketOutput(d.bytes()?.into()),
			TerminalInput => Self::TerminalInput(d.bytes()?.into()),
			TerminalOutput => Self::TerminalOutput(d.bytes()?.into()),
		})
	}
}

impl Encode for Message {
	fn encode<W: Write>(
		&self,
		e: &mut Encoder<W>,
	) -> Result<(), EncodeError<W::Error>> {
		e.encode(self.message_type())?;

		match self {
			Self::Authenticate(data) => { e.str(data.as_str())?; },
			Self::Authentication(data) => { e.bool(*data)?; },
			Self::ChildDeath(data) => { e.u8(*data)?; },
			Self::ConnectionType(data) => { e.encode(data)?; },
			Self::Hello(m, i) => { e.u8(*m)?; e.u8(*i)?; },
			Self::SignalWinch(w, h) => { e.u16(*w)?; e.u16(*h)?; },
			Self::SocketInput(data) => { e.bytes(data)?; },
			Self::SocketOutput(data) => { e.bytes(data)?; },
			Self::TerminalInput(data) => { e.bytes(data)?; },
			Self::TerminalOutput(data) => { e.bytes(data)?; },
			_ => (),
		}

		Ok(())
	}
}
