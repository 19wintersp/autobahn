#[cfg(not(target_os = "windows"))]
mod platform {
	use std::io::{ self, Error, ErrorKind };
	use std::sync::Mutex;

	use lazy_static::lazy_static;

	use libc::{
		BRKINT, CS8, CSIZE, ECHO, ECHONL, ICANON, ICRNL, IEXTEN, IGNBRK, IGNCR,
		INLCR, ISIG, ISTRIP, IXON, OPOST, PARENB, PARMRK, STDIN_FILENO, TCSAFLUSH,
		TIOCGWINSZ, STDOUT_FILENO,
	};

	lazy_static! {
		static ref TERM_CFG: Mutex<Option<libc::termios>> = Mutex::new(None);
	}

	pub unsafe fn enable_raw_mode() -> io::Result<()> {
		#[cfg(target_os = "linux")]
		let mut term_cfg = libc::termios {
			c_iflag: 0, c_oflag: 0, c_cflag: 0, c_lflag: 0,
			c_line: 0, c_cc: [ 0; 32 ],
			c_ispeed: 0, c_ospeed: 0,
		};

		#[cfg(target_os = "macos")]
		let mut term_cfg = libc::termios {
			c_iflag: 0, c_oflag: 0, c_cflag: 0, c_lflag: 0,
			c_cc: [ 0; 20 ],
			c_ispeed: 0, c_ospeed: 0,
		};

		if libc::tcgetattr(STDIN_FILENO, &mut term_cfg) == -1 {
			return Err(Error::last_os_error())
		}

		if (*TERM_CFG.lock().unwrap()).is_none() {
			*TERM_CFG.lock().unwrap() = Some(term_cfg);
		}

		term_cfg.c_iflag &= !(
			IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON
		);
		term_cfg.c_oflag &= !OPOST;
		term_cfg.c_lflag &= !(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
		term_cfg.c_cflag &= !(CSIZE | PARENB);
		term_cfg.c_cflag |= CS8;

		if libc::tcsetattr(STDIN_FILENO, TCSAFLUSH, &term_cfg) == -1 {
			Err(Error::last_os_error())
		} else {
			Ok(())
		}
	}

	pub unsafe fn disable_raw_mode() -> io::Result<()> {
		if let Some(term_cfg) = *TERM_CFG.lock().unwrap() {
			if libc::tcsetattr(STDIN_FILENO, TCSAFLUSH, &term_cfg) == -1 {
				Err(Error::last_os_error())
			} else {
				Ok(())
			}
		} else {
			warn!("term cfg uninit");
			Err(ErrorKind::Other.into())
		}
	}

	pub unsafe fn term_size() -> io::Result<(u16, u16)> {
		let mut size = libc::winsize {
			ws_row: 0, ws_col: 0,
			ws_xpixel: 0, ws_ypixel: 0,
		};

		if libc::ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut size) == -1 {
			Err(Error::last_os_error())
		} else {
			Ok((size.ws_col, size.ws_row))
		}
	}
}

#[cfg(target_os = "windows")]
mod platform {
	use std::io::{ self, Error, ErrorKind };
	use std::sync::atomic::{ AtomicU32, Ordering };

	use winapi::um::consoleapi::{ GetConsoleMode, SetConsoleMode };
	use winapi::um::handleapi::INVALID_HANDLE_VALUE;
	use winapi::um::processenv::GetStdHandle;
	use winapi::um::winbase::{ STD_INPUT_HANDLE, STD_OUTPUT_HANDLE };
	use winapi::um::wincon::{
		GetConsoleScreenBufferInfo, ENABLE_VIRTUAL_TERMINAL_INPUT,
		ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
		ENABLE_VIRTUAL_TERMINAL_PROCESSING, ENABLE_PROCESSED_OUTPUT,
		CONSOLE_SCREEN_BUFFER_INFO,
	};

	static STDIN_CFG: AtomicU32 = AtomicU32::new(0);
	static STDOUT_CFG: AtomicU32 = AtomicU32::new(0);

	pub unsafe fn enable_raw_mode() -> io::Result<()> {
		let handle = GetStdHandle(STD_INPUT_HANDLE);
		if handle == INVALID_HANDLE_VALUE {
			return Err(Error::last_os_error())
		}

		let mut term_cfg = 0;
		if GetConsoleMode(handle, &mut term_cfg) == 0 {
			return Err(Error::last_os_error())
		}

		if STDIN_CFG.load(Ordering::SeqCst) == 0 {
			STDIN_CFG.store(term_cfg, Ordering::SeqCst);
		}

		term_cfg |= ENABLE_VIRTUAL_TERMINAL_INPUT;
    term_cfg &= !(
			ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT
		);

		if SetConsoleMode(handle, term_cfg) == 0 {
			return Err(Error::last_os_error())
		}

		let handle = GetStdHandle(STD_OUTPUT_HANDLE);
		if handle == INVALID_HANDLE_VALUE {
			return Err(Error::last_os_error())
		}

		let mut term_cfg = 0;
		if GetConsoleMode(handle, &mut term_cfg) == 0 {
			return Err(Error::last_os_error())
		}

		if STDOUT_CFG.load(Ordering::SeqCst) == 0 {
			STDOUT_CFG.store(term_cfg, Ordering::SeqCst);
		}

		term_cfg |= ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING;

		if SetConsoleMode(handle, term_cfg) == 0 {
			return Err(Error::last_os_error())
		}

		Ok(())
	}

	pub unsafe fn disable_raw_mode() -> io::Result<()> {
		let term_cfg = STDIN_CFG.load(Ordering::SeqCst);
		if term_cfg == 0 {
			warn!("term cfg uninit");
			return Err(ErrorKind::Other.into())
		}

		let handle = GetStdHandle(STD_INPUT_HANDLE);
		if handle == INVALID_HANDLE_VALUE {
			return Err(Error::last_os_error())
		}

		if SetConsoleMode(handle, term_cfg) == 0 {
			return Err(Error::last_os_error())
		}

		let term_cfg = STDOUT_CFG.load(Ordering::SeqCst);
		if term_cfg == 0 {
			warn!("term cfg uninit");
			return Err(ErrorKind::Other.into())
		}

		let handle = GetStdHandle(STD_OUTPUT_HANDLE);
		if handle == INVALID_HANDLE_VALUE {
			return Err(Error::last_os_error())
		}

		if SetConsoleMode(handle, term_cfg) == 0 {
			return Err(Error::last_os_error())
		}

		Ok(())
	}

	pub unsafe fn term_size() -> io::Result<(u16, u16)> {
		let handle = GetStdHandle(STD_OUTPUT_HANDLE);
		if handle == INVALID_HANDLE_VALUE {
			return Err(Error::last_os_error())
		}

		let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
		if GetConsoleScreenBufferInfo(handle, &mut info) == 0 {
			return Err(Error::last_os_error())
		}

		Ok((info.dwSize.X as u16, info.dwSize.Y as u16))
	}
}

pub use platform::*;
