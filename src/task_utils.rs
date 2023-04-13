use std::{collections::{HashMap, BTreeMap}};
use serde::{Serialize, Deserialize, Deserializer};

use crate::{process::Status, task::Task};
use crate::{Process};

#[macro_export]
macro_rules! print_process {
	($proc_name:expr, $proc_status:expr) => {
		println!("{:<15}\t-\t{}", $proc_name, $proc_status);
	};
	($proc_name:expr, $proc_status:expr, $proc_pid:expr) => {
		println!("{:<15}\t-\t{:<23}\t-\t{}", $proc_name, $proc_status, $proc_pid);
	};
	($proc_name:expr, $proc_status:expr, $proc_pid:expr, $proc_uptime:expr) => {
		println!("{:<15}\t-\t{:<23}\t-\t{}\t-\t{}", $proc_name, $proc_status, $proc_pid, $proc_uptime);
	};
}

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
	#[serde(default = "default_numprocs")]
	pub numprocs: u32,
	#[serde(default = "default_umask")]
	#[serde(deserialize_with = "umask_deserializer")]
	pub umask: u32,
	#[serde(default = "default_workingdir")]
	pub workingdir: String,
	#[serde(default = "default_autostart")]
	pub autostart: bool,
	#[serde(default = "default_autorestart")]
	pub autorestart: Autorestart,
	#[serde(default = "default_exitcodes")]
	pub exitcodes: Vec<i32>,
	#[serde(default = "default_startretries")]
	pub startretries: u32,
	#[serde(default = "default_starttime")]
	pub starttime: u32,
	#[serde(default = "default_stopsignal")]
	pub stopsignal: Sigtype,
	#[serde(default = "default_stoptime")]
	pub stoptime: u32,
	pub stdout: Option<String>,
	pub stderr: Option<String>,
	pub env: Option<BTreeMap<String, String>>,
}

fn umask_deserializer<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

	u32::from_str_radix(&s.parse::<String>().map_err(serde::de::Error::custom)?, 8).map_err(serde::de::Error::custom)
}

fn default_numprocs() -> u32 {
	1
}

fn default_umask() -> u32 {
	19
}

fn default_workingdir() -> String {
	".".to_string()
}

fn default_autostart() -> bool {
	true
}

fn default_exitcodes() -> Vec<i32> {
	vec![0]
}

fn default_autorestart() -> Autorestart {
	Autorestart::Unexpected
}

fn default_startretries() -> u32 {
	3
}

fn default_starttime() -> u32 {
	1
}

fn default_stopsignal() -> Sigtype {
	Sigtype::TERM
}

fn default_stoptime() -> u32 {
	10
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
		if let Some(stdout) = &task.stdout {
			println!("\tNormal Output: {}", stdout);
		}
		if let Some(stderr) = &task.stderr {
			println!("\tError Output: {}", stderr);
		}
		if let Some(env) = &task.env {
			println!("\tEnv: ");
			for (key, value) in env {
				println!("\t\t- {}: {}", key, value);
			}
		}
	}
}
