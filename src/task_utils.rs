use std::{collections::{BTreeMap}};
use serde::{Serialize, Deserialize};

use crate::{Process};

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

pub fn sigtype_to_string(sigtype: &Sigtype) -> &'static str {
    match sigtype {
        Sigtype::HUP => "HUP",
        Sigtype::INT => "INT",
        Sigtype::QUIT => "QUIT",
        Sigtype::ILL => "ILL",
        Sigtype::TRAP => "TRAP",
        Sigtype::ABRT => "ABRT",
        Sigtype::EMT => "EMT",
        Sigtype::FPE => "FPE",
        Sigtype::KILL => "KILL",
        Sigtype::BUS => "BUS",
        Sigtype::SEGV => "SEGV",
        Sigtype::SYS => "SYS",
        Sigtype::PIPE => "PIPE",
        Sigtype::ALRM => "ALRM",
        Sigtype::TERM => "TERM",
        Sigtype::URG => "URG",
        Sigtype::STOP => "STOP",
        Sigtype::TSTP => "TSTP",
        Sigtype::CONT => "CONT",
        Sigtype::CHLD => "CHLD",
        Sigtype::TTIN => "TTIN",
        Sigtype::TTOU => "TTOU",
        Sigtype::IO => "IO",
        Sigtype::XCPU => "XCPU",
        Sigtype::XFSZ => "XFSZ",
        Sigtype::VTALRM => "VTALRM",
        Sigtype::PROF => "PROF",
        Sigtype::WINCH => "WINCH",
        Sigtype::INFO => "INFO",
        Sigtype::USR1 => "USR1",
        Sigtype::USR2 => "USR2",
    }
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

pub fn print_tasks(processes: &Vec<Process>) {
	println!("Printing processes:");
    for process in processes {
		println!("----------------------------------------------------------");
		if let Some(child) = &process.child {
			println!("{}		{:?}		pid {}", process.task_name, process.status, child.id());
		} else {
			println!("{}		{:?}", process.task_name, process.status);
		}
		// println!("Status: {:?}", process.status);
		// println!("----------------------------------------------------------");
	}
}
