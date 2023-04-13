use std::{process::{Child, Command}, time::Instant, fmt::Octal};
use libc::{self, mode_t, S_IRUSR, S_IWUSR, umask};
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
        if task.error.is_none() {
            let old_umask = self.set_umask(task.config.umask);
            match task.cmd.spawn() {
                Ok(child) => {
                    self.status = Status::Starting;
                    self.timer = Instant::now();
                    self.child = Some(child);
                }
                Err(error) => {println!("error: {}", error)}// add option err in process to display in status ?
            }
            self.set_umask(old_umask);
        } else {
            self.status = Status::Fatal;
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
    
    fn set_umask(&self, new_umask: libc::mode_t) -> mode_t {
        let old_umask = unsafe { umask(new_umask) };
        old_umask
    }
}