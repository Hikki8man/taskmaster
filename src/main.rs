mod task_utils;
mod terminal;

use task_utils::print_tasks;
use task_utils::Task;
use std::path::PathBuf;
use std::time::Duration;
use std::{fs::File, process::exit};
use std::io::{Read, self};
use std::{env, process};
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::process::{Command, Stdio, Child};

use crate::task_utils::Autorestart;
use crate::task_utils::print_processes;
use crate::terminal::read_input;

#[derive(Debug)]
pub struct Process {
    child: Vec<Child>,
    task: Task,
    cmd: Command,
    status: Status,
}
#[derive(Copy, Clone, PartialEq)]
enum CommandName {
	START,
	STOP,
    RESTART,
    STATUS,
}
#[derive(Debug)]
enum Status {
    Starting,
    Running,
    Stopping,
    Stopped,
    Restarting,
}
pub struct Message {
    cmd_input: CmdInput,
    status_update: Option<Status>,
}
#[derive(Clone)]
struct CmdInput {
	name: CommandName,
	arg: String,
}

impl Process {
    fn new(task: Task, cmd: Command) -> Process {
        Process { child: Vec::new(), task, cmd, status: Status::Stopped }
    }
}

macro_rules! print_exit {
	($err_msg:expr, $err_code:expr) => {
		println!("{}", $err_msg);
		exit($err_code);
	};
}

fn execute_cmd(cmd: CmdInput, process: &mut Process, sender: Sender<Message>) {
    match cmd.name {
        CommandName::START => {
            let numprocs: usize = process.task.numprocs as usize;
            if process.child.len() == numprocs {
                println!("{} already running", cmd.arg);
                return;
            }
            while process.child.len() < numprocs {
                process.child.push(process.cmd.spawn().expect("msg"));
            }
            let start_time = process.task.starttime.into();
            let cmd_clone = cmd.clone();
            sender.send(Message { cmd_input: cmd_clone, status_update: Some(Status::Starting) }).expect("msg");
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(start_time));
                sender.send(Message { cmd_input: cmd, status_update: Some(Status::Running) }).expect("msg");
            });
        }
        CommandName::STOP => {
            if process.child.is_empty() {
                println!("{} is not running", cmd.arg);
                return;
            }
            let mut i = 0;
            let mut kill = Command::new("kill");
            process.status = Status::Starting;
            while i < process.child.len() {
                kill.args(["-s", "TERM", process.child[i].id().to_string().as_str()]);
                let mut pid = kill.spawn().expect("msg");
                pid.wait().expect("msg");
                process.child.remove(i);
                i += 1;
            }
            let cmd_clone = cmd.clone();
            sender.send(Message { cmd_input: cmd_clone, status_update: Some(Status::Stopping) }).expect("msg");
            let stop_time = process.task.stoptime.into();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(stop_time));
                sender.send(Message { cmd_input: cmd, status_update: Some(Status::Stopped) }).expect("msg");
            });
        }
        CommandName::RESTART => {
            let cmd_clone = cmd.clone();

            sender.send(Message { cmd_input: cmd_clone, status_update: Some(Status::Restarting) }).expect("msg");
            if !process.child.is_empty() {
                let mut kill = Command::new("kill");
                let mut i = 0;
                process.status = Status::Starting;
                while i < process.child.len() {
                    kill.args(["-s", "TERM", process.child[i].id().to_string().as_str()]);
                    let mut pid = kill.spawn().expect("msg");
                    pid.wait().expect("msg");
                    process.child.remove(i);
                    i += 1;
                }
            }
            let numprocs: usize = process.task.numprocs as usize;

            while process.child.len() < numprocs {
                process.child.push(process.cmd.spawn().expect("msg"));
            }
            let restart_time = process.task.stoptime + process.task.starttime;
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(restart_time.into()));
                sender.send(Message { cmd_input: cmd, status_update: Some(Status::Running) }).expect("msg");
            });
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
		
	// let tasks: std::collections::HashMap<String, Task> =
	//	 serde_yaml::from_str(content.as_str()).unwrap();
	// let result: Result<std::collections::HashMap<String, Task>, serde_yaml::Error> =
	// 	serde_yaml::from_str(content.as_str());
	let tasks: std::collections::BTreeMap<String, Task>;
	match serde_yaml::from_str(content.as_str()) {
		Ok(results) => {
			tasks = results;
		},
		Err(e) => {
				print_exit!(format!("Configuration file error: {}", e), 1);
		}
	}

	//print tasks data
	// print_tasks(&tasks);
  
    let mut processes: std::collections::HashMap<String, Process> = std::collections::HashMap::new();
    let (tx, rx): (Sender<Message>, Receiver<Message>) = mpsc::channel();

    for(name, task) in tasks {

        let mut vec = task.cmd.split_whitespace();
        let stdout = File::create(task.stdout.as_str()).unwrap();
        let stderr = File::create(task.stderr.as_str()).unwrap();
        let cmd_str = vec.next().expect("msg");
        let mut cmd = Command::new(cmd_str);
        cmd.stdout(stdout);
        cmd.stderr(stderr);
        cmd.args(vec);
        cmd.current_dir(task.workingdir.as_str());

        let mut process = Process::new(task, cmd);
        if process.task.autostart {
            execute_cmd(CmdInput { name: CommandName::START, arg: String::from(&name) }, &mut process, tx.clone());
        }
        processes.insert(name, process);
    }

    let sender = tx.clone();
    let th = thread::spawn(move || {
        read_input(sender);
    });

    loop {
        for (name, mut process) in processes.iter_mut() {
            let mut i = 0;
            while i < process.child.len() {
                match process.child[i].try_wait() {
                    Ok(Some(status)) => {
                        // println!("exited with: {status}");
                        process.child.remove(i);
                        match process.task.autorestart {
                            Autorestart::Always => process.child.push(process.cmd.spawn().expect("iuiui")),
                            Autorestart::Unexpected => {
                                if !process.task.exitcodes.contains(&status.code().unwrap()) {
                                    process.child.push(process.cmd.spawn().expect("iuiui"));
                                }
                            }
                            Autorestart::Never => {}
                        }
                    }
                    Ok(None) => {}
                    Err(e) => println!("error attempting to wait: {e}"),
                }
                i += 1;
            }
        }
        let res = rx.try_recv();
        match res {
            Ok(msg) => {
                if let Some(status_update) = msg.status_update {
                    if let Some(mut proc) = processes.get_mut(msg.cmd_input.arg.as_str()) {
                        proc.status = status_update;
                        // println!("{} is now {:?}", msg.cmd_input.arg, proc.status);
                    }
                }
                else {
                    if msg.cmd_input.name == CommandName::STATUS {
                        print_processes(&processes);
                    }
                    else if let Some(mut proc) = processes.get_mut(msg.cmd_input.arg.as_str()) {
                        execute_cmd(msg.cmd_input, &mut proc, tx.clone());
                    } else {
                        println!("task not found");
                    }
                }
            }
            _ => {},
        }
    }

    th.join().expect("");
}
