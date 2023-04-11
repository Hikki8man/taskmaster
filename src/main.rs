mod task_utils;
mod terminal;
mod process;
mod task;
mod monitor;

use process::Process;
use task::Task;
use task_utils::{print_processes, print_config};
use task_utils::Config;
use std::path::PathBuf;
use std::{fs::File, process::exit};
use std::io::{Read};
use std::{env};
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::process::{Command};

use crate::monitor::Monitor;
use crate::terminal::TermInput;
use crate::terminal::read_input;

macro_rules! print_exit {
	($err_msg:expr, $err_code:expr) => {
		println!("{}", $err_msg);
		exit($err_code);
	};
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

	let mut file = File::open("tasks.yaml")
		.expect("Could not open file...");
	let mut content = String::new();
	file.read_to_string(&mut content)
		.expect("Could not read file...");
		
	let config: std::collections::BTreeMap<String, Config>;
	match serde_yaml::from_str(content.as_str()) {
		Ok(results) => {
			config = results;
		},
		Err(e) => {
			print_exit!(format!("Configuration file error: {}", e), 1);
		}
	}

	print_config(&config);
  
    let mut tasks: std::collections::HashMap<String, Task> = std::collections::HashMap::new();
    let mut processes: Vec<Process> = vec![];
    let (sender, receiver): (Sender<TermInput>, Receiver<TermInput>) = mpsc::channel();

    let mut id = 0;
    for(name, config) in config {

        let mut vec = config.cmd.split_whitespace();
        let stdout = File::create(config.stdout.as_str()).unwrap();
        let stderr = File::create(config.stderr.as_str()).unwrap();
        let cmd_str = vec.next().expect("msg");
        let mut cmd = Command::new(cmd_str);
        cmd.stdout(stdout);
        cmd.stderr(stderr);
        cmd.args(vec);
        cmd.current_dir(config.workingdir.as_str());

        let mut task = Task::new(config, cmd, name.clone());
        
        for _i in 0..task.config.numprocs {
            id += 1;
            let mut process = Process::new(id, name.clone());
            if task.config.autostart {
                process.start(&mut task);
            }
            processes.push(process);
        }
        tasks.insert(name, task);
    }

    let _th = thread::spawn(move || {
        read_input(sender);
    });
    print_processes(&processes);
    let mut monitor = Monitor::new(processes, tasks, receiver);
    monitor.task_manager_loop();
}
