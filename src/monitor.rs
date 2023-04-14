use std::{collections::HashMap, sync::{mpsc::Receiver, Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}, process::{ExitStatus, exit}, os::unix::process::ExitStatusExt, ffi::c_int};

use libc::{SIGHUP, signal};
pub static RELOAD: AtomicBool = AtomicBool::new(false);

use crate::{process::{Process, Status, self}, task::{Task, self}, task_utils::{Autorestart}, terminal::{TermInput, ProcessArg}, print_process};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CommandName {
	START,
	STOP,
    RESTART,
    STATUS,
	SHUTDOWN,
}

pub struct Monitor {
	processes: Vec<Process>,
	tasks: HashMap<String, Task>,
	receiver: Receiver<TermInput>,
	config_path: String,
	shutdown: bool,
}

impl Monitor {
	pub fn new(processes: Vec<Process>, tasks: HashMap<String, Task>, receiver: Receiver<TermInput>, config_path: String) -> Monitor {
		// self::reload = Arc::new(AtomicBool::new(false));
		unsafe { signal(SIGHUP, Self::handle_sighup_signal as usize)};
		let monitor = Monitor { processes, tasks, receiver, config_path, shutdown: false };
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
			if self.shutdown {
				if self.count_active_processes() == 0 {
					exit(0);
				} else {
					continue;
				}
			}
			self.receive_terminal_command();
		}
	}

	fn receive_terminal_command(&mut self) {
		match self.receiver.try_recv() {
			Ok(msg) => {
				// let task = self.tasks.get_mut(&msg.cmd_name);
				let cmd: CommandName = msg.cmd_name;
				let args: Vec<ProcessArg> = msg.args;
				match cmd {
					CommandName::START => {
						for arg in args {
							if let Some(task) = self.tasks.get_mut(arg.name.as_str()) {
								task.start(&mut self.processes, arg.id);
							} else {
								eprintln!("Process not found");
							}
						}
					}
					CommandName::STOP => {
						for arg in args {
							if let Some(task) = self.tasks.get_mut(arg.name.as_str()) {
								task.stop(&mut self.processes, arg.id);
							} else {
								eprintln!("Process not found");
							}
						}
					}
					CommandName::RESTART => {
						for arg in args {
							if let Some(task) = self.tasks.get_mut(arg.name.as_str()) {
								task.restart(&mut self.processes, arg.id);
							} else {
								eprintln!("Process not found");
							}
						}
					}
					CommandName::STATUS => {
						self.print_processes();
					}
					CommandName::SHUTDOWN => {
						println!("Shutting down . . .");
						self.shutdown = true;
						for (_, task) in &mut self.tasks {
							task.stop(&mut self.processes, String::from("*"));
						}
					}
				}
			}
			Err(_) => {},
		}
	}

	fn count_active_processes(&self) -> usize {
		self.processes.iter().filter(|p| p.status != Status::Stopped && p.status != Status::Fatal).count()
	}

	pub fn print_processes(&self) {
		println!("[Task Name]\t-\t[Status]\t-\t[Info]\t-\t[Uptime]");
		println!("------------------------------------------------------------------------");
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
				let uptime = process.uptime.elapsed();
				let uptime_formatted = format!(
					"{:02}:{:02}:{:02}", 
					uptime.as_secs() / 3600, 
					(uptime.as_secs() / 60) % 60, 
					uptime.as_secs() % 60
				);
				if process.status == Status::Running {
					print_process!(format, status, child.id(), uptime_formatted);
				} else {
					print_process!(format, status, child.id());
				}
			} else {
				print_process!(format, status);
			}
		}
	}
	
}