use std::collections::{HashMap, BTreeMap};
use serde::{Serialize, Deserialize};

use crate::{Process, Status};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Autorestart {
	Always,
	Unexpected,
	Never,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Sigtype {
	HUP,
	INT,
	QUIT,
	ILL,
	TRAP,
	ABRT,
	EMT,
	FPE,
	KILL,
	BUS,
	SEGV,
	SYS,
	PIPE,
	ALRM,
	TERM,
	URG,
	STOP,
	TSTP,
	CONT,
	CHLD,
	TTIN,
	TTOU,
	IO,
	XCPU,
	XFSZ,
	VTALRM,
	PROF,
	WINCH,
	INFO,
	USR1,
	USR2,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Task {
	pub cmd: String,
	pub numprocs: u32,
	pub umask: String,
	pub workingdir: String,
	pub autostart: bool,
	pub autorestart: Autorestart,
	pub exitcodes: Vec<i32>,
	pub startretries: u32,
	pub starttime: u32,
	pub stopsignal: Sigtype,
	pub stoptime: u32,
	pub stdout: String,
	pub stderr: String,
	pub env: Option<BTreeMap<String, String>>,
}

pub fn print_tasks(tasks: &BTreeMap<String, Task>) {
    for (name, task) in tasks {
		println!("App: {}", name);
		println!("\tStart Command: {}", task.cmd);
		println!("\tNumber of Processes: {}", task.numprocs);
		println!("\tUmask: {}", task.umask);
		println!("\tWorking Directory: {}", task.workingdir);
		println!("\tAutostart: {}", task.autostart);
		println!("\tAutorestart: {:?}", task.autorestart);
		println!("\tExitcodes:");
		for code in &task.exitcodes {
			println!("\t\t- {}", code);
		}
		println!("\tStart Retries: {}", task.startretries);
		println!("\tStart Time: {}", task.starttime);
		println!("\tStop Signal: {:?}", task.stopsignal);
		println!("\tStop Time: {}", task.stoptime);
		println!("\tNormal Output: {}", task.stdout);
		println!("\tError Output: {}", task.stderr);
		if let Some(env) = &task.env {
			println!("\tEnv: ");
			for (key, value) in env {
				println!("\t\t- {}: {}", key, value);
			}
		}
	}
}

pub fn print_processes(processes: &HashMap<String, Process>) {
	// println!("Task List:");
	println!("[Task Name]\t-\t[Status]");
	println!("--------------------------------");
    for (name, process) in processes {
		let status = match &process.status {
			Status::Running => "\x1B[32mRunning\x1B[0m",
			Status::Stopping => "\x1B[31mStopping\x1B[0m",
			Status::Stopped => "\x1b[30mStopped\x1B[0m",
			Status::Restarting => "\x1B[33mRestarting\x1B[0m",
			_ => "\x1B[33mStarting\x1B[0m",
		};
		println!("{:<10}\t-\t{}", name, status);
	}
}
