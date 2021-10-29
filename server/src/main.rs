#[macro_use] extern crate clap;
#[macro_use] extern crate log;

mod proxy;
mod websocket;
mod netstat;
mod config;
mod port;

use std::{ env, thread };
use std::io::{ self, Write };
use std::process::exit;
use std::str::FromStr;
use std::sync::mpsc;

use log::LevelFilter as LogLevelFilter;

use simple_logger::SimpleLogger;

const PROXY_PORT: u16 = 3321u16;
const SERVER_PORT: u16 = 3322u16;

fn main() {	
	let matches = clap_app!(app =>
		(name: env!("CARGO_PKG_NAME"))
		(version: env!("CARGO_PKG_VERSION"))
		(author: env!("CARGO_PKG_AUTHORS"))
		(about: env!("CARGO_PKG_DESCRIPTION"))
		(@setting ColoredHelp)
		(@arg CONFIG: -c --config +takes_value "Specify a configuration file instead of ~/$REPL_SLUG/.replit")
		(@arg KEY: -k --key +takes_value "Specify an environment variable to fetch the key from")
		(@arg PORT: -p --port +takes_value "Specify a port to forward to instead of detecting automatically")
		(@arg verbose: -v conflicts_with[trace] "Log more debug information to output")
		(@arg very_verbose: --verbose conflicts_with[verbose] "Log even more debug information to output")
		(@arg trace: --trace +hidden conflicts_with[very_verbose] "Log an excessive amount of debug information to output")
	).get_matches();

	SimpleLogger::new()
		.with_level(if matches.is_present("trace") {
			LogLevelFilter::Trace
		} else if matches.is_present("very_verbose") {
			LogLevelFilter::Debug
		} else if matches.is_present("verbose") {
			LogLevelFilter::Info
		} else {
			LogLevelFilter::Error
		})
		.init()
		.expect("Failed to initialize logging");
	
	info!("logging initialized");
	
	if env::var("REPL_SLUG").is_err() {
		warn!("REPL_SLUG variable not detected, are we running on Replit?");
	}

	let key_var = matches.value_of("KEY")
		.unwrap_or("KEY");
	let config_file = matches.value_of("CONFIG")
		.map(|s| s.to_string())
		.or(
			env::var("HOME").ok()
				.zip(env::var("REPL_SLUG").ok())
				.map(|(home, slug)| format!("{}/{}/.replit", home, slug))
		);
	let port = matches.value_of("PORT");

	if let Ok(key) = env::var(key_var) {
		info!("running as server");

		let port = if let Some(port_str) = port {
			if let Ok(port) = u16::from_str(port_str) {
				Some(port)
			} else {
				error!("port argument invalid");
				exit(1);
			}
		} else {
			config_file
				.and_then(|config_file| config::load_config(config_file.as_str()))
				.and_then(|config| config.port)
				.or_else(|| port::get_port_auto())
		};

		if let Some(port) = port {
			info!("starting intercepting port {}", port);
		} else {
			warn!("no port detected, not proxying");
		}

		let (proxy_shutdown, proxy_signal) = mpsc::channel();
		let (server_shutdown, server_signal) = mpsc::channel();
		
		thread::spawn(move || proxy::start(port, proxy_signal));
		thread::spawn(move || websocket::start(key.as_str(), server_signal));

		println!("Press <ENTER> to exit");

		let stdin = io::stdin();
		let mut stdout = io::stdout();
		loop {
			let _ = stdin.read_line(&mut String::new());

			print!("Really exit? [y/N] ");
			let _ = stdout.flush();

			let mut buffer = String::new();
			if stdin.read_line(&mut buffer).is_ok() {
				if buffer.trim().to_lowercase() == "y" {
					break
				}
			}
		}

		info!("sending shutdown to proxy and server");

		if proxy_shutdown.send(()).is_err() { error!("failed to shut down proxy") }
		if server_shutdown.send(()).is_err() { error!("failed to shut down server") }

		info!("goodbye");
	} else {
		error!("{} environment variable not set", key_var);
		exit(1);
	}
}
