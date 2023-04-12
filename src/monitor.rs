use std::{collections::HashMap, sync::{mpsc::Receiver, Arc, atomic::{AtomicBool, Ordering}}, time::Duration, process::ExitStatus, os::unix::process::ExitStatusExt, ffi::c_int};

use libc::{SIGHUP, signal};
pub static RELOAD: AtomicBool = AtomicBool::new(false);

use crate::{process::{Process, Status}, task::Task, task_utils::{Autorestart, print_processes}, terminal::TermInput};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CommandName {
	START,
	STOP,
    RESTART,
    STATUS,
}

pub struct Monitor {
	processes: Vec<Process>,
	tasks: HashMap<String, Task>,
	receiver: Receiver<TermInput>
}

impl Monitor {
	pub fn new(processes: Vec<Process>, tasks: HashMap<String, Task>, receiver: Receiver<TermInput>) -> Monitor {
		// self::reload = Arc::new(AtomicBool::new(false));
		unsafe { signal(SIGHUP, Self::handle_sighup_signal as usize)};
		Monitor { processes, tasks, receiver }
	}

	fn handle_sighup_signal(_: i32) {
		RELOAD.store(true, Ordering::SeqCst);
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
	pub fn task_manager_loop(&mut self) {
		loop {
			if RELOAD.load(Ordering::SeqCst) == true {
				RELOAD.store(false, Ordering::SeqCst);
			}
			for process in self.processes.iter_mut() {
				let task = self.tasks.get_mut(process.task_name.as_str()).expect("lol");//Todo unwrapor and kill processes
				
				if let Some(child) = &mut process.child {
					match child.try_wait() {
						Ok(Some(status)) => {
							println!("exit status: {:?}, Process status: {:?}", status.code(), process.status);
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
											if status.code().is_none() || !task.config.exitcodes.contains(&status.code().unwrap_or(0)) {
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
							Monitor::check_state(process, task);
						}
						Err(e) => println!("error attempting to wait: {}", e),
					}
				}
			}
			self.receive_terminal_command();
		}
	}

	fn receive_terminal_command(&mut self) {
		match self.receiver.try_recv() {
			Ok(msg) => {
				let task = self.tasks.get_mut(&msg.arg);
				match msg.name {
					CommandName::START => {
						if let Some(task) = task {
							task.start(&mut self.processes);
						} else {
							println!("Task not found");
						}
					}
					CommandName::STOP => {
						if let Some(task) = task {
							task.stop(&mut self.processes);
						} else {
							println!("Task not found");
						}
					}
					CommandName::RESTART => {
						if let Some(task) = task {
							task.restart(&mut self.processes);
						} else {
							println!("Task not found");
						}
					}
					CommandName::STATUS => {
						print_processes(&self.processes);
					}
				}
			}
			Err(_) => {},
		}
	}
}