#[macro_use] extern crate clap;
#[macro_use] extern crate log;

mod console;
mod portfwd;
mod shell;
mod websocket;

use crate::websocket::{ Connection, ConnectionSettings, Repl };

use std::env;
use std::io::{ self, Write };
use std::process::exit;
use std::str::FromStr;

use log::LevelFilter as LogLevelFilter;

use simple_logger::SimpleLogger;

fn main() {
	let matches = clap_app!(app =>
		(name: env!("CARGO_PKG_NAME"))
		(version: env!("CARGO_PKG_VERSION"))
		(author: env!("CARGO_PKG_AUTHORS"))
		(about: env!("CARGO_PKG_DESCRIPTION"))
		(@setting ArgsNegateSubcommands)
		(@setting ColoredHelp)
		(@setting GlobalVersion)
		(@setting SubcommandsNegateReqs)
		(@setting SubcommandRequiredElseHelp)
		(@arg REPL: +takes_value +required +global "Specify the repl to connect to")
		(@arg KEY: -k --key +takes_value +global "Specify the key to authenticate with")
		(@arg verbose: -v conflicts_with[trace] +global "Log more debug information to output")
		(@arg very_verbose: --verbose conflicts_with[verbose] +global "Log even more debug information to output")
		(@arg trace: --trace +hidden conflicts_with[very_verbose] +global "Log an excessive amount of debug information to output")
		(@subcommand portfwd =>
			(about: "Listen on a local port and forward to a port in the repl")
			(@setting ColoredHelp)
			(@arg REMOTE: -r --remote +takes_value +required "Specify the remote port to forward to")
			(@arg LOCAL: -l --local +takes_value "Specify the local port to listen on")
		)
		(@subcommand shell =>
			(@setting ColoredHelp)
			(about: "Open and connect to a remote shell in the repl")
		)
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
	
	if env::var("REPL_SLUG").is_ok() {
		warn!("REPL_SLUG variable detected, why are we running on Replit?");
	}

	let repl = match value_t!(matches, "REPL", Repl) {
		Ok(repl) => repl,
		_ => {
			error!("failed to parse repl");
			exit(1);
		},
	};

	let key = match matches.value_of("KEY") {
		Some(key) => key.to_string(),
		_ => {
			print!("Password: ");
			let _ = io::stdout().flush();

			let mut password = String::new();
			if io::stdin().read_line(&mut password).is_ok() {
				password
			} else {
				error!("io error");
				exit(1);
			}
		}
	};

	let mut connection = ConnectionSettings {
		connection: Connection::Shell,
		repl, key,
	};

	if let Some(matches) = matches.subcommand_matches("portfwd") {
		let local = matches.value_of("LOCAL")
			.and_then(|string| {
				u16::from_str(string)
					.ok()
					.or_else(|| {
						warn!("invalid local port argument, using default");
						None
					})
			});
		
		let remote = u16::from_str(
			matches.value_of("REMOTE")
				.unwrap()
		)
			.unwrap_or_else(|_| {
				error!("failed to parse remote port");
				exit(1);
			});
		
		connection.connection = Connection::Port(remote);

		portfwd::start(connection, local);
	} else {
		shell::start(connection);
	}
}
