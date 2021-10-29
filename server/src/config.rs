use std::fs;

use serde_derive::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct DotReplit {
	pub autobahn: Config,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct Config {
	pub port: Option<u16>,
}

pub fn load_config(file: &str) -> Option<Config> {
	fs::read(file)
		.map_err(|_| ())
		.and_then(|data| String::from_utf8(data).map_err(|_| ()))
		.and_then(|data| toml::from_str(data.as_str()).map_err(|_| ()))
		.map(|result: DotReplit| Some(result))
		.unwrap_or(None)
		.map(|data| data.autobahn)
}
