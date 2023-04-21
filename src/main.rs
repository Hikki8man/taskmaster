mod task_utils;
mod terminal;
mod process;
mod task;
mod monitor;
mod logger;

use task_utils::Config;
use std::collections::{BTreeMap};
use std::error::Error;
use std::path::PathBuf;
use std::{fs::File, process::exit};
use std::io::{Read};
use std::{env};
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

use crate::monitor::Monitor;
use crate::terminal::{TermInput, Terminal};

macro_rules! print_exit {
	($err_msg:expr, $err_code:expr) => {
		println!("{}", $err_msg);
		exit($err_code);
	};
}

pub fn parse_config_file(path: &PathBuf) -> Result<BTreeMap<String, Config>, Box<dyn Error>> {
	let mut file = File::open(path)?;
	let mut content = String::new();
	file.read_to_string(&mut content)?;
	let configs: BTreeMap<String, Config> = serde_yaml::from_str(&content)?;
	Ok(configs)
}

fn main() {
	let default_path = PathBuf::from("tasks.yaml");
	let args: Vec<String> = env::args().collect();

	match args.len() {
		3.. => {
			print_exit!("Too many arguments. Useage: ./executable [path_to_config]", 1);
		},
		_ => {
			println!("Checking path to configuration file...");
		}
	}

	let path = env::args()
					.nth(1)
					.map(PathBuf::from)
					.unwrap_or(default_path);
	println!("{:?}", path);
	let extension = path.extension();
	if extension.is_none() || extension.unwrap() != "yaml"
	{
		print_exit!("Wrong file extention. Expecting a YAML file.", 1);
	}
	let config: BTreeMap<String, Config> = match parse_config_file(&path) {
		Ok(cfg) => cfg,
		Err(e) => { print_exit!(e, 1); }
	};

    let (sender, receiver): (Sender<TermInput>, Receiver<TermInput>) = mpsc::channel();
    let mut monitor = Monitor::new(config, receiver, path);
    let _th = thread::spawn(move || {
		let mut terminal: Terminal = Terminal::new(sender);
		terminal.read_input();
    });
    monitor.task_manager_loop();
}
