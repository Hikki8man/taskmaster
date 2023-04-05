mod task_utils;
mod terminal;
mod command;

use task_utils::print_tasks;
use task_utils::Config;
use std::path::PathBuf;
use std::{fs::File, process::exit};
use std::io::{Read};
use std::{env};
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::process::{Command, Child};

use crate::task_utils::Autorestart;
use crate::command::execute_cmd;
use crate::terminal::read_input;

#[derive(Debug)]
pub struct Task {
    processes: Vec<Process>,
    config: Config,
    cmd: Command,
}

#[derive(Debug)]
struct Process {
    id: u32,
    child: Option<Child>,
    status: Status,
    retries: u32,
}

impl Process {
    fn new(id: u32) -> Process {
        Process {
            id,
            child: None,
            status: Status::Stopped,
            retries: 0,
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
}

#[derive(Debug)]
pub struct Message {
    cmd_input: CmdInput,
    status_update: Option<StatusAndId>,
}

#[derive(Debug)]
pub struct StatusAndId {
    status: Status,
    id: u32
}

#[derive(Clone, Debug)]
pub struct CmdInput {
	name: CommandName,
	arg: String,
    from_term: bool,
}

impl Task {
    fn new(config: Config, cmd: Command) -> Task {
        let mut vec: Vec<Process> = Vec::new();
        let mut i = 0;
        while i < config.numprocs {
            vec.push(Process::new(i));
            i += 1;
        }
        Task { processes: vec, config, cmd }
    }

    fn find_process_by_id(&mut self, id: u32) -> Option<&mut Process> {
        for process in self.processes.iter_mut() {
            if process.id == id {
                return Some(process);
            }
        }
        None
    }
}

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

	//print tasks data
	// print_tasks(&tasks);
  
    let mut tasks: std::collections::HashMap<String, Task> = std::collections::HashMap::new();
    let (tx, rx): (Sender<Message>, Receiver<Message>) = mpsc::channel();

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

        let mut task = Task::new(config, cmd);
        if task.config.autostart {
            execute_cmd(CmdInput { name: CommandName::START, arg: String::from(&name), from_term: false }, &mut task, tx.clone());
        }
        tasks.insert(name, task);
    }

    let sender = tx.clone();
    let _th = thread::spawn(move || {
        read_input(sender);
    });

    loop {
        for (name, task) in tasks.iter_mut() {
            // for mut process in task.processes.iter_mut() {
            let mut i = 0;
            while i < task.processes.len() {
                if let Some(child) = &mut task.processes[i].child {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            
                            println!("exited with: {}", status);
                            task.processes[i].child = None;
                            if task.processes[i].retries >= task.config.startretries {
                                continue;
                            }
                            let cmd = CmdInput {
                                name: CommandName::START,
                                arg: name.clone(),
                                from_term: false
                            };
                            println!("{} restarting for the {} time", name, task.processes[i].retries + 1);
                            match task.config.autorestart {
                                Autorestart::Always => {
                                    task.processes[i].retries += 1;
                                    execute_cmd(cmd, task, tx.clone());
                                }
                                Autorestart::Unexpected => {
                                    if !task.config.exitcodes.contains(&status.code().unwrap_or(0)) {
                                        task.processes[i].retries += 1;
                                        execute_cmd(cmd, task, tx.clone());
                                    }
                                }
                                Autorestart::Never => {}
                            }
                        }
                        Ok(None) => {}
                        Err(e) => println!("error attempting to wait: {}", e),
                    }
                }
                i += 1;
            }
        }

        let res = rx.try_recv();
        match res {
            Ok(msg) => {
                println!("{:?}", msg);
                if let Some(status_update) = msg.status_update {
                    if let Some(task) = tasks.get_mut(msg.cmd_input.arg.as_str()) {
                        if let Some(proc) = task.find_process_by_id(status_update.id) {
                            if (status_update.status == Status::Running && proc.status != Status::Starting) ||
                            (status_update.status == Status::Stopped && proc.status != Status::Stopping) {
                                continue;
                            }
                            if status_update.status == Status::Stopped && proc.status == Status::Stopping {
                                proc.child = None;
                            }
                            println!("status received: {:?}", status_update.status);
                            proc.status = status_update.status;
                        }
                    }
                }
                else {
                    if msg.cmd_input.name == CommandName::STATUS {
                        print_tasks(&tasks);
                    }
                    else if let Some(mut task) = tasks.get_mut(msg.cmd_input.arg.as_str()) {
                        if msg.cmd_input.from_term {
                            let mut i = 0;
                            while i < task.processes.len() {
                                task.processes[i].retries = 0;
                                i += 1;
                            }
                        }
                        execute_cmd(msg.cmd_input, &mut task, tx.clone());
                    } else {
                        println!("task not found");
                    }
                }
            }
            _ => {},
        }
    }

    // th.join().expect("");
}
