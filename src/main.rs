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

//pabo
fn set_cmd_output(cmd: &mut Command, path: &Option<String>, stdout: bool) -> Result<(), io::Error> {
	if let Some(path) = path {
		match OpenOptions::new().create(true).append(true).open(path) {
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

fn create_task_and_processes(config: BTreeMap<String, Config>) -> HashMap<String, Task> {
	let mut tasks: HashMap<String, Task> = HashMap::new();
	// let mut processes: Vec<Process> = vec![];
	
	for(name, config) in config {
		
		let mut task = Task::new(config, name.clone());
		let mut cmd_splited: VecDeque<&str> = task.config.cmd.split_whitespace().collect();
		if cmd_splited.is_empty() {
			//FATAL     command is empty
			continue; //Todo
		}
        let cmd_str = cmd_splited[0];
		cmd_splited.pop_front();
		
		for id in 0..task.config.numprocs {
       		let mut cmd = Command::new(cmd_str);
			if let Some(env) = &task.config.env {
				cmd.envs(env);
			}
			cmd.args(cmd_splited.clone());
			cmd.current_dir(task.config.workingdir.as_str());

			let mut error: Option<Box<dyn Error>> = None;
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
        tasks.insert(name, task);
    }
	tasks
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
	if !path.try_exists().expect("Unable to check file existence.")
	{
		print_exit!("Invalid path.", 1);
	}
	if !path.is_file()
	{
		print_exit!("Not a file.", 1);
	}
	//TODO fix parsing
	let mut file = File::open("tasks.yaml")
		.expect("Could not open file...");
	let mut content = String::new();
	file.read_to_string(&mut content)
		.expect("Could not read file...");
		
	let config: BTreeMap<String, Config>;
	match serde_yaml::from_str(content.as_str()) {
		Ok(results) => {
			config = results;
		},
		Err(e) => {
			print_exit!(format!("Configuration file error: {}", e), 1);
		}
	}

    let (sender, receiver): (Sender<TermInput>, Receiver<TermInput>) = mpsc::channel();
	let tasks = create_task_and_processes(config);

    let mut monitor = Monitor::new(tasks, receiver, String::from("tasks.yaml")); //Todo: Get real path
    let _th = thread::spawn(move || {
		let mut terminal: Terminal = Terminal::new(sender);
		terminal.read_input();
    });
    monitor.task_manager_loop();
}
