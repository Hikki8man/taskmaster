use std::{collections::{HashMap, BTreeMap}, sync::{mpsc::Receiver, Arc, atomic::{AtomicBool, Ordering}}, process::{exit}, fs::{OpenOptions, File}, error::Error, io::Read, path::PathBuf};

use libc::{SIGHUP, signal};
pub static RELOAD: AtomicBool = AtomicBool::new(false);

use crate::{process::{Status}, task::{Task}, terminal::{TermInput, ProcessArg}, task_utils::Config, parse_config_file, create_task_and_processes};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CommandName {
	START,
	STOP,
    RESTART,
	UPDATE,
    STATUS,
	SHUTDOWN,
}

pub struct Monitor {
	tasks: HashMap<String, Task>,
	receiver: Receiver<TermInput>,
	config_path: PathBuf,
	shutdown: bool,
}

impl Monitor {
	pub fn new(tasks: HashMap<String, Task>, receiver: Receiver<TermInput>, config_path: PathBuf) -> Monitor {
		// self::reload = Arc::new(AtomicBool::new(false));
		unsafe { signal(SIGHUP, Self::handle_sighup_signal as usize)};
		let mut monitor = Monitor { tasks, receiver, config_path, shutdown: false };
		monitor.print_status(vec![]);
		return monitor;
	}

	fn handle_sighup_signal(_: i32) {
		RELOAD.store(true, Ordering::SeqCst);
	}

	pub fn task_manager_loop(&mut self) {
		loop {
			if RELOAD.load(Ordering::SeqCst) == true {
				RELOAD.store(false, Ordering::SeqCst);
			}
			for (_name, task) in self.tasks.iter_mut() {
				task.try_wait();
			}
			if self.shutdown && !self.process_still_alive() {
				exit(0);
			}
			self.receive_terminal_command();
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
								// println!("arg:{:?}", arg);
								task.start(arg.id);
							} else {
								eprintln!("Task {} not found", arg.name);
							}
						}
					}
					CommandName::STOP => {
						for arg in args {
							if let Some(task) = self.tasks.get_mut(arg.name.as_str()) {
								task.stop(arg.id);
							} else {
								eprintln!("Task {} not found", arg.name);
							}
						}
					}
					CommandName::RESTART => {
						for arg in args {
							if let Some(task) = self.tasks.get_mut(arg.name.as_str()) {
								task.restart(arg.id);
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
							task.stop("*".to_string());
						}
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
		let mut configs: BTreeMap<String, Config> = parse_config_file(&self.config_path)?;
		for (name, task) in &mut self.tasks {
			if let Some(config) = configs.remove(name) {
				if task.config != config {
					task.stop("*".to_string());
					task.wait_procs_to_stop();
					*task = create_task_and_processes(name.clone(), config).1;
				} else {
					println!("no diff");
				}
			}
		}
		// for (name, config) in configs {

		// }
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