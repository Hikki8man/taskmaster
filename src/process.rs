use std::{process::{Child, Command}, time::Instant};
use libc::{self, mode_t};
use crate::{task_utils::sigtype_to_string, task::Task};

#[derive(Debug, PartialEq, Clone)]
pub enum Status {
    Starting,
    Running,
    Stopping,
    Stopped,
    Restarting,
    Fatal,
}

#[derive(Debug)]
pub struct Process {
    pub id: u32,
    pub task_name: String,
    pub child: Option<Child>,
    pub status: Status,
    pub retries: u32,
    pub timer: Instant,
}

impl Process {
    pub fn new(id: u32, task_name: String) -> Process {
        Process {
            id,
            task_name,
            child: None,
            status: Status::Stopped,
            retries: 0,
            timer: Instant::now(),
        }
    }

    pub fn start(&mut self, task: &mut Task) {
        if let Some(_child) = &self.child {
            return println!("Process {} is already running", self.id);
        }
        // TODO test supervisor with bad cmd to see if it retry
        match task.cmd.spawn() {
            Ok(child) => {
                self.status = Status::Starting;
                self.timer = Instant::now();
                self.child = Some(child);
            }
            Err(error) => {println!("{}", error)}// add option err in process to display in status ?
        }
        self.retries += 1;
    }

    pub fn stop(&mut self, task: &mut Task) {
        if let Some(child) = &self.child {
            let mut kill_cmd = Command::new("kill");
            match kill_cmd.args(["-s", sigtype_to_string(&task.config.stopsignal), child.id().to_string().as_str()]).output() {
                Ok(_) => {
                    self.timer = Instant::now();
                    self.status = Status::Stopping;
                }
                Err(e) => {println!("{}", e)}
            }
        }
    }

    pub fn restart(&mut self, task: &mut Task) {
        self.stop(task);
        self.status = Status::Restarting;
    }

    pub fn kill(&mut self) {
        if let Some(child) = &mut self.child {
            println!("KILLLL");
            match child.kill() {
                Ok(_) => {
                    self.child = None;
                    self.status = Status::Stopped;
                }
                Err(e) => {println!("{}", e)}
            }
        }
    }
    
    fn set_umask(&self, new_umask: libc::mode_t) -> Result<mode_t, String> {
        let old_umask = unsafe { libc::umask(0) };
        if old_umask != 0 {
            return Err(String::from("Failed to get current umask"));
        }
        let result = unsafe { libc::umask(new_umask) };
        if result != 0 {
            Err(String::from("Failed to set umask"))
        } else {
            Ok(old_umask)
        }
    }
}