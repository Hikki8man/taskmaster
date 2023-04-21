use std::{collections::{HashMap, BTreeMap, VecDeque}, sync::{mpsc::Receiver, Arc, atomic::{AtomicBool, Ordering}}, process::{exit, Command, Stdio}, fs::{OpenOptions, File}, error::Error, io::{Read, self}, path::PathBuf};
use crate::{process::{Status, Process}, task::{Task}, terminal::{TermInput, ProcessArg}, task_utils::Config, parse_config_file, logger::Logger};
use libc::{SIGHUP, signal};

pub static RELOAD: AtomicBool = AtomicBool::new(false);


#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CommandName {
	START,
	STOP,
    RESTART,
	UPDATE,
    STATUS,
	SHUTDOWN,
	KILL,
}

pub struct Monitor {
	tasks: HashMap<String, Task>,
	receiver: Receiver<TermInput>,
	config_path: PathBuf,
	shutdown: bool,
	logger: Logger,
}

impl Monitor {
	pub fn new(config: BTreeMap<String, Config>, receiver: Receiver<TermInput>, config_path: PathBuf) -> Monitor {
		unsafe { signal(SIGHUP, Self::handle_sighup_signal as usize)};
		let mut tasks: HashMap<String, Task> = HashMap::new();
		let mut logger = Logger::new();
		for (name, config) in config {
			let (name, task) = Self::create_task_and_processes(name, config, &mut logger);
			tasks.insert(name, task);
		}
		let mut monitor = Monitor { tasks, receiver, config_path, shutdown: false, logger };
		monitor.print_status(vec![]);
		return monitor;
	}

	fn create_task_and_processes(name: String, config: Config, logger: &mut Logger) -> (String, Task) {
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
	
			if let Err(e) = Self::set_cmd_output(&mut cmd, &task.config.stdout, true) {
				error = Some(Box::new(e));
			}
			if let Err(e) = Self::set_cmd_output(&mut cmd, &task.config.stdout, false) {
				error = Some(Box::new(e));
			}
			let mut process = Process::new(id, name.clone(), cmd, task.config.umask, task.config.stopsignal);
			process.error = error;
			if task.config.autostart {
				process.start(logger);
			}
			task.processes.push(process);
		}
		(name, task)
	}

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

	fn handle_sighup_signal(_: i32) {
		RELOAD.store(true, Ordering::SeqCst);
	}

	pub fn task_manager_loop(&mut self) {
		loop {
			for (_name, task) in self.tasks.iter_mut() {
				task.try_wait(&mut self.logger);
			}
			if self.shutdown && !self.process_still_alive() {
				exit(0);
			}
			self.receive_terminal_command();
			if RELOAD.load(Ordering::SeqCst) == true {
				RELOAD.store(false, Ordering::SeqCst);
				match self.update() {
					Ok(()) => {},
					Err(e) => { eprintln!("{:?}", e) }
				}
			}
		}
	}

	fn receive_terminal_command(&mut self) {
		match self.receiver.try_recv() {
			Ok(msg) => {
				let cmd: CommandName = msg.cmd_name;
				let args: Vec<ProcessArg> = msg.args;
				match cmd {
					CommandName::START => {
						for arg in args {
							if let Some(task) = self.tasks.get_mut(arg.name.as_str()) {
								task.start(arg.id, &mut self.logger);
							} else {
								eprintln!("Task {} not found", arg.name);
							}
						}
					}
					CommandName::STOP => {
						for arg in args {
							if let Some(task) = self.tasks.get_mut(arg.name.as_str()) {
								task.stop(arg.id, &mut self.logger);
							} else {
								eprintln!("Task {} not found", arg.name);
							}
						}
					}
					CommandName::RESTART => {
						for arg in args {
							if let Some(task) = self.tasks.get_mut(arg.name.as_str()) {
								task.restart(arg.id, &mut self.logger);
							} else {
								eprintln!("Task {} not found", arg.name);
							}
						}
					}
					CommandName::STATUS => {
						self.print_status(args);
					}
					CommandName::UPDATE => {
						match self.update() {
							Ok(()) => {},
							Err(e) => { eprintln!("{:?}", e) }
						}
					}
					CommandName::SHUTDOWN => {
						println!("Shutting down . . .");
						self.shutdown = true;
						for (_, task) in &mut self.tasks {
							task.stop("*".to_string(), &mut self.logger);
						}
					}
					CommandName::KILL => {
						println!("Shutting down murdering all childs :( . . .");
						for (_, task) in &mut self.tasks {
							task.kill(&mut self.logger);
						}
						exit(0);
					}
				}
			}
			Err(_) => {},
		}
	}

	fn process_still_alive(&self) -> bool {
		self.tasks.iter().any(|(_, task)| {
			task.processes.iter().any(|p| {
				p.status != Status::Stopped && p.status != Status::Fatal
			})
		})
	}

	fn update(&mut self) -> Result<(), Box<dyn Error>> {
		let mut to_remove: Vec<String> = vec![];
		let mut configs: BTreeMap<String, Config> = parse_config_file(&self.config_path)?;
		for (name, task) in &mut self.tasks {
			if let Some(config) = configs.remove(name) {
				if task.config != config {
					task.stop("*".to_string(), &mut self.logger);
					task.wait_procs_to_stop(&mut self.logger);
					*task = Self::create_task_and_processes(name.clone(), config, &mut self.logger).1;
				}
			} else {
				//STOP DELETE TASK
				task.stop("*".to_string(), &mut self.logger);
				task.wait_procs_to_stop(&mut self.logger);
				to_remove.push(name.clone());
			}
		}
		for name in to_remove {
			self.tasks.remove(name.as_str());
		}
		//START HANDLE NEW TASKS
		for (name, config) in configs {
			let (name, new_task) = Self::create_task_and_processes(name, config, &mut self.logger);
			self.tasks.insert(name, new_task);
		}
		println!("Update complete");
		Ok(())
	}

	pub fn print_status(&mut self, args: Vec<ProcessArg>) {
		println!("[Task Name]\t-\t[Status]\t-\t[Info]\t-\t[Uptime]");
		println!("------------------------------------------------------------------------");
		if args.is_empty() {
			for (_name, task) in &mut self.tasks {
				task.print_processes("*".to_string());
			}
		} else {
			for arg in args {
				if let Some(task) = self.tasks.get_mut(arg.name.as_str()) {
					task.print_processes(arg.id);
				} else {
					eprintln!("Task {} not found", arg.name);
				}
			}
		}
		println!("------------------------------------------------------------------------");
	}
	
}