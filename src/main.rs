use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs::File, process::exit};
use std::io::{Read, self};
use std::{env, process};
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use serde::{Serialize, Deserialize};
use std::process::{Command, Stdio, Child};

#[derive(Serialize, Deserialize, Debug)]
struct Task {
	cmd: String,
	numprocs: u32,
	umask: String,
	workingdir: String,
	autostart: bool,
	autorestart: String,
	exitcodes: Vec<u8>,
	startretries: u32,
	starttime: u32,
	stopsignal: String,
	stoptime: u32,
	stdout: String,
	stderr: String,
	env: Option<HashMap<String, String>>,
}

#[derive(Debug)]
struct Process {
    child: Vec<Child>,
    task: Task,
    cmd: Command,
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
	let tasks: std::collections::HashMap<String, Task>;
	// let result: Result<std::collections::HashMap<String, Task>, serde_yaml::Error> =
	// 	serde_yaml::from_str(content.as_str());
	match serde_yaml::from_str(content.as_str()) {
		Ok(results) => {
			tasks = results;
		},
		Err(e) => {
				print_exit!(format!("Configuration file error: {}", e), 1);
		}
	}

	//print tasks data
	// for (name, task) in tasks {
	// 	println!("App: {}", name);
	// 	println!("\tStart Command: {}", task.cmd);
	// 	println!("\tNumber of Processes: {}", task.numprocs);
	// 	println!("\tUmask: {}", task.umask);
	// 	println!("\tWorking Directory: {}", task.workingdir);
	// 	println!("\tAutostart: {}", task.autostart);
	// 	println!("\tAutorestart: {}", task.autorestart);
	// 	println!("\tExitcodes:");
	// 	for code in task.exitcodes {
	// 		println!("\t\t- {}", code);
	// 	}
	// 	println!("\tStart Retries: {}", task.startretries);
	// 	println!("\tStart Time: {}", task.starttime);
	// 	println!("\tStop Signal: {}", task.stopsignal);
	// 	println!("\tStop Time: {}", task.stoptime);
	// 	println!("\tNormal Output: {}", task.stdout);
	// 	println!("\tError Output: {}", task.stderr);
	// 	if let Some(env) = task.env {
	// 		println!("\tEnv: ");
	// 		for (key, value) in env {
	// 			println!("\t\t- {}: {}", key, value);
	// 		}
	// 	}
	// }

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

    let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();

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

                }
                "stop" => {
                    println!("msg: {:?}", input_vec);
                    if input_vec.len() > 1 {
                        tx.send(input_vec[1].to_string()).expect("msg");
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
                        if (process.task.autostart) {// autorestart
                            process.cmd.spawn().expect("iuiui");
                        }
                    }
                    Ok(None) => {}
                    Err(e) => println!("error attempting to wait: {e}"),
                }
                i += 1;
            }
        }
        // let res = rx.recv();
        // match res {
        //     Ok(msg) => {
        //     }
        //     _ => exit(1)
        // }
    }

    th.join().expect("");
}
