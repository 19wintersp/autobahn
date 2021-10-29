use super::{ parse_ip, process, Process, SockTabEntry };

use std::net;
use std::str::FromStr;

pub fn parse_sock_tab(sock_tab: String) -> Vec<SockTabEntry> {
	let mut out: Vec<SockTabEntry> = vec![];

	let mut lines = sock_tab.split('\n');
	lines.next(); // skip title

	for line in lines {
		// ignore comments
		let line = line.get(0..line.find('#').unwrap_or(line.len())).unwrap();

		trace!("parsing sock tab line {}", line);

		let fields: Vec<&str> = line.split_whitespace().collect();

		if fields.len() < 12 {
			warn!("line did not have enough fields");

			continue;
		}

		let local_addr: Option<net::SocketAddr>;
		let remote_addr: Option<net::SocketAddr>;
		let state: Option<u8>;
		let uid: Option<u32>;
		let process: Option<Process>;

		if let Ok(addr) = parse_ip::parse(fields[1]) {
			local_addr = Some(addr);
		} else {
			warn!("couldn't parse local addr");
			continue;
		}

		if let Ok(addr) = parse_ip::parse(fields[2]) {
			remote_addr = Some(addr);
		} else {
			warn!("couldn't parse remote addr");
			continue;
		}

		if let Ok(sstate) = u8::from_str_radix(fields[3], 16) {
			state = Some(sstate);
		} else {
			warn!("couldn't parse state");
			continue;
		}

		if let Ok(suid) = u32::from_str(fields[7]) {
			uid = Some(suid);
		} else {
			warn!("couldn't parse uid");
			continue;
		}

		if let Ok(sproc) = process::get_info(fields[9].to_string()) {
			process = Some(sproc);
		} else {
			warn!("couldn't get process info");
			continue;
		}

		out.push(
			SockTabEntry {
				ino: fields[9].to_string(),
				local_addr: local_addr.unwrap(),
				remote_addr: remote_addr.unwrap(),
				state: state.unwrap(),
				uid: uid.unwrap(),
				process: process.unwrap(),
			}
		);
	}

	out
}
