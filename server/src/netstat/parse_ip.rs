use std::net::{ SocketAddr, Ipv6Addr, SocketAddrV6 };

pub fn parse(ip: &str) -> Result<SocketAddr, ()> {
	parse_v4(ip).or_else(|_| parse_v6(ip))
}

pub fn parse_v4(sock: &str) -> Result<SocketAddr, ()> {
	if sock.len() != 13 {
		warn!("failed to parse socket which was wrong size");
		return Err(());
	}

	let split = sock.find(':');
	if split.is_none() {
		warn!("failed to parse socket without port");
		return Err(());
	}

	let (ip, port) = sock.split_at(split.unwrap());

	if let Ok(ip_val) = u64::from_str_radix(ip, 16) {
		let d = ip_val % 256;
		let c = (ip_val / 256) % 256;
		let b = (ip_val / 256 / 256) % 256;
		let a = (ip_val / 256 / 256 / 256) % 256;

		if let Ok(port_val) = u16::from_str_radix(port.get(1..).unwrap(), 16) {
			let sock_str = format!("{}.{}.{}.{}:{}", a, b, c, d, port_val);
			sock_str.as_str().parse()
				.map_err(|_| warn!("failed to parse socket {}", sock_str))
		} else {
			warn!("failed to parse port {}", port);
			Err(())
		}
	} else {
		warn!("failed to parse ip {}", ip);
		Err(())
	}
}

pub fn parse_v6(sock: &str) -> Result<SocketAddr, ()> {
	if sock.len() != 37 {
		warn!("failed to parse socket which was wrong size");
		return Err(());
	}

	let split = sock.find(':');
	if split.is_none() {
		warn!("failed to parse socket without port");
		return Err(());
	}

	let (ip, port) = sock.split_at(split.unwrap());
	
	if let Ok(mut ip_val) = u128::from_str_radix(ip, 16) {
		let h = ip_val % 256;
		ip_val /= 256;
		let g = ip_val % 256;
		ip_val /= 256;
		let f = ip_val % 256;
		ip_val /= 256;
		let e = ip_val % 256;
		ip_val /= 256;
		let d = ip_val % 256;
		ip_val /= 256;
		let c = ip_val % 256;
		ip_val /= 256;
		let b = ip_val % 256;
		ip_val /= 256;
		let a = ip_val % 256;

		let ip_addr = Ipv6Addr::new(
			a as u16,
			b as u16,
			c as u16,
			d as u16,
			e as u16,
			f as u16,
			g as u16,
			h as u16,
		);

		if let Ok(port_val) = u16::from_str_radix(port.get(1..).unwrap(), 16) {
			Ok(SocketAddr::V6(SocketAddrV6::new(ip_addr, port_val, 0, 0)))
		} else {
			warn!("failed to parse port {}", port);
			Err(())
		}
	} else {
		warn!("failed to parse ip {}", ip);
		Err(())
	}
}
