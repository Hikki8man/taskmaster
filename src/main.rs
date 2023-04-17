mod task_utils;
mod terminal;
mod process;
mod task;
mod monitor;

use process::Process;
use task::Task;
use task_utils::{print_config};
use task_utils::Config;
use std::collections::{HashMap, BTreeMap, VecDeque};
use std::error::Error;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::{fs::File, process::exit};
use std::io::{Read, self};
use std::{env};
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::process::{Command, Stdio};

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

//pabo
fn set_cmd_output(cmd: &mut Command, path: &Option<String>, stdout: bool) -> Result<(), io::Error> {
	if let Some(path) = path {
		match OpenOptions::new().create(true).append(true).write(true).open(path) {
			Ok(file) => {
				if stdout {
					cmd.stdout(file);
				} else {
					cmd.stderr(file);
				}
				Ok(())
			}
			Err(e) => {
				if stdout {
					cmd.stdout(Stdio::null());
				} else {
					cmd.stderr(Stdio::null());
				}
				Err(e)
			}
		}
	} else {
		if stdout {
			cmd.stdout(Stdio::null());
		} else {
			cmd.stderr(Stdio::null());
		}
		Ok(())
	}
}

fn create_task_and_processes(name: String, config: Config) -> (String, Task) {
	// let mut tasks: HashMap<String, Task> = HashMap::new();
	// let mut processes: Vec<Process> = vec![];
	
	// for(name, config) in config {
		
		let mut task = Task::new(config, name.clone());
		let cmd_split: VecDeque<&str> = task.config.cmd.split_whitespace().collect();
		
		for id in 0..task.config.numprocs {
			let mut error: Option<Box<dyn Error>> = None;
			let mut cmd_splited = cmd_split.clone();
			let mut cmd = match cmd_splited.pop_front() {
				Some(cmd_str) => Command::new(cmd_str),
				None => {
					error = Some(Box::new(io::Error::new(io::ErrorKind::Other, "Command is empty")));
					Command::new("")
				}
			};
			if let Some(env) = &task.config.env {
				cmd.envs(env);
			}
			cmd.args(cmd_splited.clone());
			cmd.current_dir(task.config.workingdir.as_str());

			if let Err(e) = set_cmd_output(&mut cmd, &task.config.stdout, true) {
				error = Some(Box::new(e));
			}
			if let Err(e) = set_cmd_output(&mut cmd, &task.config.stdout, false) {
				error = Some(Box::new(e));
			}
			let mut process = Process::new(id, name.clone(), cmd, task.config.umask);
			process.error = error;
            if task.config.autostart {
                process.start();
            }
            task.processes.push(process);
        }
        // tasks.insert(name, task);
    // }
	(name, task)
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
	let config = match parse_config_file(&path) {
		Ok(cfg) => cfg,
		Err(e) => { print_exit!(e, 1); }
	};

	// if !path.try_exists().expect("Unable to check file existence.")
	// {
	// 	print_exit!("Invalid path.", 1);
	// }
	// if !path.is_file()
	// {
	// 	print_exit!("Not a file.", 1);
	// }
	// //TODO fix parsing
	// let mut file = File::open("tasks.yaml")
	// 	.expect("Could not open file...");
	// let mut content = String::new();
	// file.read_to_string(&mut content)
	// 	.expect("Could not read file...");
    let (sender, receiver): (Sender<TermInput>, Receiver<TermInput>) = mpsc::channel();
	let mut tasks: HashMap<String, Task> = HashMap::new();
	for (name, config) in config {
		let (name, task) = create_task_and_processes(name, config);
		tasks.insert(name, task);
	}

    let mut monitor = Monitor::new(tasks, receiver, path); //Todo: Get real path
    let _th = thread::spawn(move || {
		let mut terminal: Terminal = Terminal::new(sender);
		terminal.read_input();
    });
    monitor.task_manager_loop();
}
