use std::{collections::HashMap, sync::{mpsc::Receiver, Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}, process::ExitStatus, os::unix::process::ExitStatusExt, ffi::c_int};

use libc::{SIGHUP, signal};
pub static RELOAD: AtomicBool = AtomicBool::new(false);

use crate::{process::{Process, Status}, task::Task, task_utils::{Autorestart}, terminal::TermInput, print_process};

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
	receiver: Receiver<TermInput>,
	config_path: String,
}

impl Monitor {
	pub fn new(processes: Vec<Process>, tasks: HashMap<String, Task>, receiver: Receiver<TermInput>, config_path: String) -> Monitor {
		// self::reload = Arc::new(AtomicBool::new(false));
		unsafe { signal(SIGHUP, Self::handle_sighup_signal as usize)};
		let monitor = Monitor { processes, tasks, receiver, config_path };
		monitor.print_processes();
		return monitor;
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
					process.uptime = Instant::now();
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
						self.print_processes();
					}
				}
			}
			Err(_) => {},
		}
	}

	pub fn print_processes(&self) {
		println!("[Task Name]\t-\t[Status]\t-\t[PID]");
		println!("------------------------------------------------------");
		for process in &self.processes {
			let task = self.tasks.get(&process.task_name).unwrap();
			let status = match &process.status {
				Status::Running => "\x1B[32mRunning\x1B[0m",
				Status::Stopping => "\x1B[31mStopping\x1B[0m",
				Status::Stopped => "\x1b[30mStopped\x1B[0m",
				Status::Restarting => "\x1B[33mRestarting\x1B[0m",
				Status::Fatal => "\x1B[31mFatal\x1B[0m",
				_ => "\x1B[33mStarting\x1B[0m",
			};
			let format = if self.processes.len() > 1 { format!("{}:{}", process.task_name, process.id) }
				else { process.task_name.clone() };
			if let Some(err) = &task.error {
				print_process!(format, status, err);
			}
			else if let Some(child) = &process.child {
				print_process!(format, status, child.id());
			} else {
				print_process!(format, status);
			}
		}
	}
	
}