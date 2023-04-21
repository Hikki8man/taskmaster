use std::{collections::{BTreeMap}, fmt};
use serde::{Serialize, Deserialize, Deserializer};

#[macro_export]
macro_rules! print_process {
	($proc_name:expr, $proc_status:expr) => {
		println!("{:<15.15}\t-\t{}", $proc_name, $proc_status);
	};
	($proc_name:expr, $proc_status:expr, $proc_pid:expr) => {
		println!("{:<15.15}\t-\t{:<23}\t-\t{}", $proc_name, $proc_status, $proc_pid);
	};
	($proc_name:expr, $proc_status:expr, $proc_pid:expr, $proc_uptime:expr) => {
		println!("{:<15.15}\t-\t{:<23}\t-\t{}\t-\t{}", $proc_name, $proc_status, $proc_pid, $proc_uptime);
	};
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Autorestart {
	Always,
	Unexpected,
	Never,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum Sigtype {
    HUP = 1,
    INT = 2,
    QUIT = 3,
    ILL = 4,
    TRAP = 5,
    ABRT = 6,
    BUS = 7,
    FPE = 8,
    KILL = 9,
    USR1 = 10,
    SEGV = 11,
    USR2 = 12,
    PIPE = 13,
    ALRM = 14,
    TERM = 15,
    STKFLT = 16,
    CHLD = 17,
    CONT = 18,
    STOP = 19,
    TSTP = 20,
    TTIN = 21,
    TTOU = 22,
    URG = 23,
    XCPU = 24,
    XFSZ = 25,
    VTALRM = 26,
    PROF = 27,
    WINCH = 28,
    POLL = 29,
    PWR = 30,
    SYS = 31,
}

impl fmt::Display for Sigtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sig_str = match self {
            Sigtype::HUP => "HUP",
            Sigtype::INT => "INT",
            Sigtype::QUIT => "QUIT",
            Sigtype::ILL => "ILL",
            Sigtype::TRAP => "TRAP",
            Sigtype::ABRT => "ABRT",
            Sigtype::BUS => "BUS",
            Sigtype::FPE => "FPE",
            Sigtype::KILL => "KILL",
            Sigtype::USR1 => "USR1",
            Sigtype::SEGV => "SEGV",
            Sigtype::USR2 => "USR2",
            Sigtype::PIPE => "PIPE",
            Sigtype::ALRM => "ALRM",
            Sigtype::TERM => "TERM",
            Sigtype::STKFLT => "STKFLT",
            Sigtype::CHLD => "CHLD",
            Sigtype::CONT => "CONT",
            Sigtype::STOP => "STOP",
            Sigtype::TSTP => "TSTP",
            Sigtype::TTIN => "TTIN",
            Sigtype::TTOU => "TTOU",
            Sigtype::URG => "URG",
            Sigtype::XCPU => "XCPU",
            Sigtype::XFSZ => "XFSZ",
            Sigtype::VTALRM => "VTALRM",
            Sigtype::PROF => "PROF",
            Sigtype::WINCH => "WINCH",
            Sigtype::POLL => "POLL",
            Sigtype::PWR => "PWR",
            Sigtype::SYS => "SYS",
        };
        write!(f, "{}", sig_str)
    }
}

impl From<i32> for Sigtype {
    fn from(value: i32) -> Self {
        match value {
            1 => Sigtype::HUP,
            2 => Sigtype::INT,
            3 => Sigtype::QUIT,
            4 => Sigtype::ILL,
            5 => Sigtype::TRAP,
            6 => Sigtype::ABRT,
            7 => Sigtype::BUS,
            8 => Sigtype::FPE,
            9 => Sigtype::KILL,
            10 => Sigtype::USR1,
            11 => Sigtype::SEGV,
            12 => Sigtype::USR2,
            13 => Sigtype::PIPE,
            14 => Sigtype::ALRM,
            15 => Sigtype::TERM,
            16 => Sigtype::STKFLT,
            17 => Sigtype::CHLD,
            18 => Sigtype::CONT,
            19 => Sigtype::STOP,
            20 => Sigtype::TSTP,
            21 => Sigtype::TTIN,
            22 => Sigtype::TTOU,
            23 => Sigtype::URG,
            24 => Sigtype::XCPU,
            25 => Sigtype::XFSZ,
            26 => Sigtype::VTALRM,
            27 => Sigtype::PROF,
            28 => Sigtype::WINCH,
            29 => Sigtype::POLL,
            30 => Sigtype::PWR,
            31 => Sigtype::SYS,
            _ => Sigtype::KILL,//
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
