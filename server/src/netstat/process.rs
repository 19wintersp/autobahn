use super::Process;

use std::fs;
use std::str::FromStr;

const PROC_DIR: &str = "/proc";

#[derive(Clone, Debug)]
struct ProcFd {
	pub inode: String,
	pub base: String,
	pub pid: i32,
}

pub fn get_info(inode: String) -> Result<Process, ()> {
	fs::read_dir(PROC_DIR).map_err(|_| ())
		.and_then(|dir| {
			for _file in dir {
				if let Ok(file) = _file {
					if let Ok(file_type) = file.file_type() {
						if !file_type.is_dir() {
							continue;
						}

						let file_name = file.file_name();
						let file_name = file_name.to_str();
						if file_name.is_none() { continue; }
						let file_name = file_name.unwrap();

						trace!("processing {}", file_name);

						if let Ok(pid) = i32::from_str(file_name) {
							let ctx = ProcFd {
								inode: inode.clone(),
								base: format!("{}/{}", PROC_DIR, file_name),
								pid: pid,
							};

							if let Some(process) = process_proc_dir(ctx) {
								return Ok(process);
							}
						}
					}
				}
			}

			trace!("ran out of proc files");
			Err(())
		})
}

fn process_proc_dir(ctx: ProcFd) -> Option<Process> {
	let fd_dir = format!("{}/{}", ctx.base, "fd");

	if let Ok(dir) = fs::read_dir(fd_dir.clone()) {
		for _file in dir {
			if let Ok(file) = _file {
				if let Ok(link_target) = fs::read_link(
					format!("{}/{}", fd_dir, file.file_name().to_str().unwrap())
				) {
					trace!("processing link target {:?}", link_target.to_str());

					let link_target_string = match link_target.to_str() {
						Some(string) => string,
						_ => continue,
					};

					if format!("socket:[{}]", ctx.inode) != link_target_string {
						trace!("socket didn't match {}", ctx.inode);
						continue;
					}

					if let Ok(stat_data) = fs::read(format!("{}/{}", ctx.base, "stat"))
						.map_err(|_| ())
						.and_then(|data| String::from_utf8(data).map_err(|_| ()))
					{
						let parts: Vec<&str> = stat_data.split_whitespace().collect();
						let proc_name = get_proc_name(parts[1].to_string());

						return Some(
							Process {
								pid: ctx.pid,
								name: proc_name,
							}
						);
					} else {
						warn!("failed to read stat file");
					}
				} else {
					warn!("couldn't follow symlink");
				}
			}
		}

		trace!("ran out of files");
		None
	} else {
		trace!("directory read nulled");
		None
	}
}

fn get_proc_name(string: String) -> String {
	string.get((string.find('(').unwrap_or(0) + 1)..string.rfind(')').unwrap_or(0)).unwrap().into()
}
