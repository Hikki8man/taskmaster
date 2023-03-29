use std::{collections::{BTreeMap, HashMap}, process};
use serde::{Serialize, Deserialize};

use crate::Task;

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
pub struct Config {
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

pub fn print_config(tasks: &BTreeMap<String, Config>) {
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

pub fn print_tasks(tasks: &HashMap<String, Task>) {
    for (name, task) in tasks {
		println!("----------------------------------------------------------");
		println!("Task: {} ------", name);
		for process in &task.processes {
			println!("Status: {:?}", process.status);
		}
		println!("----------------------------------------------------------");
	}
}