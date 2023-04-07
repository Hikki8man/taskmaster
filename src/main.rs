mod task_utils;
mod terminal;
mod command;

use task_utils::print_tasks;
use task_utils::Config;
use task_utils::sigtype_to_string;
// use std::os::linux::process;
use std::path::PathBuf;
use std::process;
use std::time::Duration;
use std::time::Instant;
use std::{fs::File, process::exit};
use std::io::{Read};
use std::{env};
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::process::{Command, Child};

use crate::task_utils::Autorestart;
// use crate::command::execute_cmd;
use crate::terminal::read_input;

#[derive(Debug)]
pub struct Task {
    // processes: Vec<Process>,
    name: String,
    config: Config,
    cmd: Command,
}

#[derive(Debug)]
pub struct Process {
    id: u32,
    task_name: String,
    child: Option<Child>,
    status: Status,
    retries: u32,
    timer: Instant,
}

impl Process {
    fn new(id: u32, task_name: String) -> Process {
        Process {
            id,
            task_name,
            child: None,
            status: Status::Stopped,
            retries: 0,
            timer: Instant::now(),
        }
    }

    fn start(&mut self, task: &mut Task) {
        if let Some(_child) = &self.child {
            return println!("Process {} is already running", self.id);
        }
        match task.cmd.spawn() {
            Ok(child) => {
                self.status = Status::Starting;
                self.timer = Instant::now();
                self.child = Some(child);
            }
            Err(error) => {}// add option err in process to display in status ?
        }
        self.retries += 1;
    }

    fn stop(&mut self, task: &mut Task) {
        if let Some(child) = &self.child {
            let mut kill_cmd = Command::new("kill");
            match kill_cmd.args(["-s", sigtype_to_string(&task.config.stopsignal), child.id().to_string().as_str()]).output() {
                Ok(_) => {
                    self.timer = Instant::now();
                    self.status = Status::Stopping;
                }
                Err(e) => {println!("{}", e)}
            }
        }
    }

    fn restart(&mut self, task: &mut Task) {
        self.stop(task);
        self.status = Status::Restarting;
    }

    fn kill(&mut self) {
        if let Some(child) = &mut self.child {
            println!("KILLLL");
            match child.kill() {
                Ok(_) => {
                    self.child = None;
                    self.status = Status::Stopped;
                }
                Err(e) => {println!("{}", e)}
            }
        }
    }

}
#[derive(Copy, Clone, PartialEq, Debug)]
enum CommandName {
	START,
	STOP,
    RESTART,
    STATUS,
}

#[derive(Debug, PartialEq, Clone)]
enum Status {
    Starting,
    Running,
    Stopping,
    Stopped,
    Restarting,
    Fatal,
}

#[derive(Clone, Debug)]
pub struct TermInput {
	name: CommandName,
	arg: String,
}

impl Task {
    fn new(config: Config, cmd: Command, name: String) -> Task {
        Task { config, cmd, name }
    }

    fn start(&mut self, processes: &mut Vec<Process>) {
        for process in processes {
            if process.task_name == self.name {
                process.retries = 0;
                process.start(self);
            }
        }
    }

    fn stop(&mut self, processes: &mut Vec<Process>) {
        for process in processes {
            if process.task_name == self.name {
                process.stop(self);
            }
        }
    }

    fn restart(&mut self, processes: &mut Vec<Process>) {
        for process in processes {
            if process.task_name == self.name {
                process.retries = 0;
                process.restart(self);
            }
        }
    }
}

macro_rules! print_exit {
	($err_msg:expr, $err_code:expr) => {
		println!("{}", $err_msg);
		exit($err_code);
	};
}

fn check_state(process: &mut Process, task: &mut Task) {
    match process.status {
        Status::Starting => {
            if process.timer.elapsed() > Duration::new(task.config.starttime as u64, 0) {
                process.retries = 0;
                process.status = Status::Running;
                println!("{}:{} is now running", process.task_name, process.id);
            }
        }
        Status::Stopping => {
            if process.timer.elapsed() > Duration::new(task.config.stoptime as u64, 0) {
                process.kill();
                println!("{}:{} is now stopped", process.task_name, process.id);
            }
        }
        Status::Restarting => {
            if process.timer.elapsed() > Duration::new(task.config.stoptime as u64, 0) {
                process.kill();
                process.start(task);
            }
        }
        _ => {}
    }
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

	//print tasks data
	// print_tasks(&tasks);
  
    let mut tasks: std::collections::HashMap<String, Task> = std::collections::HashMap::new();
    let mut processes: Vec<Process> = vec![];
    let (tx, rx): (Sender<TermInput>, Receiver<TermInput>) = mpsc::channel();

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

    let sender = tx.clone();
    let _th = thread::spawn(move || {
        read_input(sender);
    });
    print_tasks(&processes);
    loop {
        for process in processes.iter_mut() {
            let task = tasks.get_mut(process.task_name.as_str()).expect("lol");//Todo unwrapor and kill processes
            
            if let Some(child) = &mut process.child {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        println!("exit status: {}, Process status: {:?}", status, process.status);
                        process.child = None;
                        match process.status {
                            Status::Starting => {
                                if process.retries < task.config.startretries {
                                    process.start(task);
                                } else {
                                    process.status = Status::Fatal;
                                }
                            }
                            Status::Stopping => {
                                process.status = Status::Stopped;
                            }

                            Status::Restarting => {
                                process.start(task);
                            }

                            _ => {
                                match task.config.autorestart {
                                    Autorestart::Always => {
                                        process.start(task);
                                    }
                                    Autorestart::Unexpected => {
                                        if !task.config.exitcodes.contains(&status.code().unwrap_or(0)) {
                                            process.start(task);
                                        } else {
                                            process.status = Status::Stopped;
                                        }
                                    }
                                    Autorestart::Never => { process.status = Status::Stopped }
                                }
                            }

                        }
                    }
                    Ok(None) => {
                        check_state(process, task);
                    }
                    Err(e) => println!("error attempting to wait: {}", e),
                }
            }
        }
        match rx.try_recv() {
            Ok(msg) => {
                let task = tasks.get_mut(&msg.arg);
                match msg.name {
                    CommandName::START => {
                        if let Some(task) = task {
                            task.start(&mut processes);
                        } else {
                            println!("Task not found");
                        }
                    }
                    CommandName::STOP => {
                        if let Some(task) = task {
                            task.stop(&mut processes);
                        } else {
                            println!("Task not found");
                        }
                    }
                    CommandName::RESTART => {
                        if let Some(task) = task {
                            task.restart(&mut processes);
                        } else {
                            println!("Task not found");
                        }
                    }
                    CommandName::STATUS => {
                        print_tasks(&processes);
                    }
                }
            }
            Err(_) => {},
        }
    }
}
