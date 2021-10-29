use super::{ parse, SockTabEntry };

use std::fs;

pub fn do_netstat(sock_tab_path: &str) -> Vec<SockTabEntry> {
	let mut out: Vec<SockTabEntry> = vec![];

	debug!("doing netstat");

	// read file and parse
	if let Ok(tab_data) = fs::read(sock_tab_path).map_err(|_| ())
		.and_then(|data| String::from_utf8(data).map_err(|_| ()))
	{
		trace!("parsing sock tab");

		out.append(&mut parse::parse_sock_tab(tab_data));
	} else {
		warn!("failed to read sock tab");

		return vec![];
	}

	out
}
