use crate::netstat;

/* impl fmt::Debug for netstat::SockTabEntry {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SockTabEntry")
			.field("ino", &self.ino)
			.field("local_addr", &self.local_addr)
			.field("remote_addr", &self.remote_addr)
			.field("state", &self.state)
			.field("uid", &self.uid)
			.field("process.name", &self.process.name)
			.field("process.pid", &self.process.pid)
			.finish()
	}
} */

pub fn get_port_auto() -> Option<u16> {
	trace!("fetching addrs");

	let mut _addrs: Vec<netstat::SockTabEntry> = vec![];
	_addrs.append(&mut netstat::os_socks(netstat::SockType::TCP));
	_addrs.append(&mut netstat::os_socks(netstat::SockType::TCP6));

	trace!("filtering addrs");

	let mut addrs: Vec<netstat::SockTabEntry> = vec![];
	for addr in _addrs.clone() {
		let listening = addr.state == 10;
		let unspecified = addr.local_addr.ip().is_unspecified();
		let loopback = addr.local_addr.ip().is_loopback();

		if listening && (loopback || unspecified) {
			addrs.push(addr);
		}
	}

	if addrs.len() == 0 {
		None
	} else if addrs.len() == 1 {
		Some(addrs[0].local_addr.port())
	} else {
		println!("{} listeners detected:", addrs.len());

		let mut index = 1;
		for addr in addrs.clone() {
			println!(
				"{}: {} ({}) - [{}]:{}",
				index,
				addr.process.name,
				addr.process.pid,
				addr.local_addr.ip(),
				addr.local_addr.port()
			);

			index += 1;
		}

		error!("TODO: this needs fixing!!!");
		unimplemented!();

		/* loop {
			if let Some(choice) = input::prompt("Which one should be used?".to_string())
				.and_then(
					|input| usize::from_str(input.as_str())
						.map(|ok| Some(ok - 1))
						.unwrap_or(None)
				)
			{
				return Some(addrs[choice].local_addr.port());
			}
		} */
	}
}
