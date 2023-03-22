mod task_utils;

use task_utils::print_tasks;
use task_utils::Task;
use std::path::PathBuf;
use std::{fs::File, process::exit};
use std::io::{Read, self};
use std::{env, process};
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::process::{Command, Stdio, Child};

use crate::task_utils::Autorestart;

#[derive(Debug)]
struct Process {
    child: Vec<Child>,
    task: Task,
    cmd: Command,
}

enum CommandName {
	START,
	STOP,
}

struct Cmd {
	name: CommandName,
	arg: String,
}

impl Process {
    fn new(task: Task, cmd: Command) -> Process {
        Process { child: Vec::new(), task, cmd }
    }
}

macro_rules! print_exit {
	($err_msg:expr, $err_code:expr) => {
		println!("{}", $err_msg);
		exit($err_code);
	};
}

fn execute_cmd(cmd: Cmd, process: &mut Process) {
    match cmd.name {
        CommandName::START => {
            let numprocs: usize = process.task.numprocs as usize;
            if process.child.len() == numprocs {
                println!("{} already running", cmd.arg);
                return;
            }
            println!("starting {} ...", cmd.arg);
            let mut i = 0;
            while process.child.len() < numprocs {
                process.child.push(process.cmd.spawn().expect("msg"));
                i += 1;
            }
        }
        CommandName::STOP => {
            println!("stopping {} ...", cmd.arg);
            let mut i = 0;
            while i < process.child.len() {
                process.child[i].kill();
                process.child.remove(i);
                i += 1;
            }
        }
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
	print_tasks(&tasks);
  
    let mut processes: std::collections::HashMap<String, Process> = std::collections::HashMap::new();

    for(name, task) in tasks {

        let mut vec = task.cmd.split_whitespace();
        let output = File::create("output.txt").unwrap();
        let cmd_str = vec.next().expect("msg");
        let mut cmd = Command::new(cmd_str);
        cmd.stdout(Stdio::from(output));
        cmd.args(vec);

        let mut process = Process::new(task, cmd);
        if process.task.autostart {
            let mut i = 0;
            while i < process.task.numprocs {
                println!("number of pro: {}", process.task.numprocs);
                let child = process.cmd.spawn().expect("msg");
                process.child.push(child);
                i += 1;
            }
        }
        processes.insert(name, process);
    }

    let (tx, rx): (Sender<Cmd>, Receiver<Cmd>) = mpsc::channel();

    let th = thread::spawn(move || {
        loop {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).expect("msg");
            let input_vec: Vec<&str> = buffer.split_whitespace().collect();
            println!("input: {:?}", input_vec);
            if input_vec.is_empty() {
                continue;
            }
            match input_vec[0] {
                "start" => {
                    if input_vec.len() > 1 {
						let cmd: Cmd = Cmd { name: CommandName::START, arg: String::from(input_vec[1]) };
                        tx.send(cmd).expect("msg");
                    }
                }
                "stop" => {
                    if input_vec.len() > 1 {
						let cmd: Cmd = Cmd { name: CommandName::STOP, arg: String::from(input_vec[1]) };
                        tx.send(cmd).expect("msg");
                    }
                }
                "exit" => {
                    break;
                }
                _ => {

                }
            }
            // tx.send(input_trimed).expect("msg");
        }
    });

    loop {
        for (name, mut process) in processes.iter_mut() {
            let mut i = 0;
            while i < process.child.len() {
                match process.child[i].try_wait() {
                    Ok(Some(status)) => {
                        println!("exited with: {status}");
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
            Ok(cmd) => {
                if let Some(mut proc) = processes.get_mut(cmd.arg.as_str()) {
                    execute_cmd(cmd, &mut proc);
                } else {
                    println!("process not found");
                }
            }
            _ => {},
        }
    }

    th.join().expect("");
}
