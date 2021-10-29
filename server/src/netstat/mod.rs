mod netstat;
mod parse;
mod parse_ip;
mod process;

use std::net;

const TCP_TAB_PATH: &str = "/proc/net/tcp";
const TCP6_TAB_PATH: &str = "/proc/net/tcp6";
const UDP_TAB_PATH: &str = "/proc/net/udp";
const UDP6_TAB_PATH: &str = "/proc/net/udp6";

#[derive(Clone, Debug)]
pub struct Process {
	pub pid: i32,
	pub name: String,
}

#[derive(Clone, Debug)]
pub struct SockTabEntry {
	pub ino: String,
	pub local_addr: net::SocketAddr,
	pub remote_addr: net::SocketAddr,
	pub state: u8,
	pub uid: u32,
	pub process: Process,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SockType {
	TCP,
	TCP6,
	UDP,
	UDP6,
}

pub fn os_socks(sock_type: SockType) -> Vec<SockTabEntry> {
	let tab_path = match sock_type {
		SockType::TCP => TCP_TAB_PATH,
		SockType::TCP6 => TCP6_TAB_PATH,
		SockType::UDP => UDP_TAB_PATH,
		SockType::UDP6 => UDP6_TAB_PATH,
	};

	trace!("tab path is {}", tab_path);

	netstat::do_netstat(tab_path)
}
