use super::{ Input, Output };

use std::thread;
use std::fs::File;
use std::sync::mpsc::{ self, Receiver, Sender };
use std::io::{ self, Read, Write };
use std::os::unix::io::{ FromRawFd, RawFd };

use libc::{ SIGCONT, SIGSTOP, SIGWINCH, SIGKILL, TIOCSWINSZ };

pub(super) fn handle_client() -> io::Result<(Sender<Input>, Receiver<Output>)> {
	let (pty_fd, child_pid) = unsafe { launch_process("/bin/bash") }?;
	let mut pty = unsafe { File::from_raw_fd(pty_fd) };
	let mut pty_clone = pty.try_clone().map_err(|err| err)?;

	let (input_tx, input_rx) = mpsc::channel();
	let (output_tx, output_rx) = mpsc::channel();

	let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>();
	let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>();

	thread::spawn(move || loop {
		let mut buffer = [ 0; 256 ];
		if let Ok(read) = pty.read(&mut buffer) {
			if read == 0 {
				break
			} else {
				let _ = read_tx.send(buffer[..read].to_vec());
			}
		}
	});

	thread::spawn(move || loop {
		if let Ok(data) = write_rx.recv() {
			let _ = pty_clone.write(&data);
		}
	});

	thread::spawn(move || loop {
		if let Ok(data) = read_rx.try_recv() {
			let _ = output_tx.send(Output::Data(data));
		}

		if let Ok(exit) = unsafe { exit_status(child_pid) } {
			if let Some(status) = exit {
				let _ = output_tx.send(Output::Died(status));

				break
			}
		} else {
			warn!("failed to get child info");
		}

		if let Ok(input) = input_rx.try_recv() {
			match input {
				Input::Data(data) => {
					let _ = write_tx.send(data);
				},
				Input::Continue => unsafe { libc::kill(child_pid, SIGCONT); },
				Input::Stop => unsafe { libc::kill(child_pid, SIGSTOP); },
				Input::Winch(w, h) => unsafe {
					let size = libc::winsize {
						ws_row: h,
						ws_col: w,
						ws_xpixel: 0,
						ws_ypixel: 0,
					};

					if libc::ioctl(pty_fd, TIOCSWINSZ, &size) != -1 {
						libc::kill(child_pid, SIGWINCH);
					}
				},
				Input::End => unsafe {
					libc::kill(child_pid, SIGKILL);

					break
				},
			}
		}
	});

	Ok((input_tx, output_rx))
}

unsafe fn launch_process(path: &str) -> io::Result<(RawFd, libc::pid_t)> {
	use std::ffi::CStr;
	use std::os::raw::c_ulong;
	use std::ptr::null;

	use libc::{ O_NOCTTY, O_RDWR, TIOCSCTTY };

	const TIOCNOTTY: c_ulong = 0x5422; // for some reason this isn't in libc

	let pty_master = libc::posix_openpt(O_NOCTTY | O_RDWR);
	if pty_master == -1 {
		return Err(io::Error::last_os_error())
	}

	if libc::grantpt(pty_master) == -1 {
		return Err(io::Error::last_os_error())
	}

	if libc::unlockpt(pty_master) == -1 {
		return Err(io::Error::last_os_error())
	}

	let pty_slave_path = libc::ptsname(pty_master);
	if pty_slave_path.is_null() {
		return Err(io::Error::last_os_error())
	}

	let pty_slave = libc::open(pty_slave_path, O_RDWR);
	if pty_slave == -1 {
		return Err(io::Error::last_os_error())
	}

	let fork_result = libc::fork();
	if fork_result == -1 {
		return Err(io::Error::last_os_error())
	} else if fork_result == 0 {
		let ctty_file = CStr::from_bytes_with_nul(b"/dev/tty\0").unwrap();

		let pty_current = libc::open(ctty_file.as_ptr(), O_NOCTTY | O_RDWR);
		if pty_current != -1 {
			if libc::ioctl(pty_current, TIOCNOTTY, 0) == -1 {
				return Err(io::Error::last_os_error())
			}

			libc::close(pty_current);
		}

		if libc::setsid() == -1 {
			return Err(io::Error::last_os_error())
		}

		if libc::ioctl(pty_slave, TIOCSCTTY, 0) == -1 {
			return Err(io::Error::last_os_error())
		}

		if libc::dup2(pty_slave, 0) == -1 {
			return Err(io::Error::last_os_error())
		}
		if libc::dup2(pty_slave, 1) == -1 {
			return Err(io::Error::last_os_error())
		}
		if libc::dup2(pty_slave, 2) == -1 {
			return Err(io::Error::last_os_error())
		}

		libc::close(pty_current);

		let path_nullterm = [ path.as_bytes(), &[ 0 ] ].concat();
		let path = CStr::from_bytes_with_nul(&path_nullterm).unwrap();
		let env_nullterm = [ "TERM=xterm-256color".as_bytes(), &[ 0 ] ].concat();
		let env = CStr::from_bytes_with_nul(&env_nullterm).unwrap();
		libc::execve(
			path.as_ptr(),
			[ path.as_ptr(), null() ].as_ptr(),
			[ env.as_ptr(), null() ].as_ptr(), // fix this pls
		);

		return Err(io::Error::last_os_error())
	};

	Ok((pty_master, fork_result))
}

unsafe fn exit_status(pid: libc::pid_t) -> io::Result<Option<u8>> {
	use libc::WNOHANG;

	let mut status = 0;
	match libc::waitpid(pid, &mut status, WNOHANG) {
		-1 => Err(io::Error::last_os_error()),
		0 => Ok(None),
		_ => if libc::WIFEXITED(status) {
			Ok(Some(libc::WEXITSTATUS(status) as u8))
		} else {
			Ok(Some(255)) // we probably need a better mechanism for this
		},
	}
}
