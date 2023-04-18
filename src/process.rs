use std::{process::{Child, Command}, time::{Instant, Duration}, error::Error};
use libc::{self, mode_t, umask};
use crate::{task_utils::{sigtype_to_string, Sigtype, Config}};

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
    cmd: Command,
    umask: u32,
    task_name: String,
    stop_sig: Sigtype,
    pub child: Option<Child>,
    pub status: Status,
    pub retries: u32,
    pub timer: Instant,
    pub uptime: Instant,
    pub error: Option<Box<dyn Error>>,
}

impl Process {
    pub fn new(id: u32, task_name: String, cmd: Command, umask: u32, stop_sig: Sigtype) -> Process {
        Process {
            id,
            cmd,
            umask,
            stop_sig,
            child: None,
            task_name,
            status: Status::Stopped,
            retries: 0,
            timer: Instant::now(),
            uptime: Instant::now(),
            error: None
        }
    }

    pub fn start(&mut self) {
        if let Some(_child) = &self.child {
            return println!("Process {}:{} is already running", self.task_name, self.id);
        }
        if self.error.is_none() {
            let old_umask = self.set_umask(self.umask);
            match self.cmd.spawn() {
                Ok(child) => {
                    self.status = Status::Starting;
                    self.timer = Instant::now();
                    self.child = Some(child);
                }
                Err(error) => { self.error = Some(Box::new(error)) }
            }
            self.set_umask(old_umask);
        } else {
            self.status = Status::Fatal;
        }
        self.retries += 1;
    }

    pub fn stop(&mut self) {
        if self.status == Status::Stopping { return; }
        if let Some(child) = &self.child {
            let mut kill_cmd = Command::new("kill");
            match kill_cmd.args(["-s", sigtype_to_string(&self.stop_sig), child.id().to_string().as_str()]).output() {
                Ok(_) => {
                    self.timer = Instant::now();
                    self.status = Status::Stopping;
                }
                Err(e) => { eprintln!("{}", e) }
            }
        }
    }

    pub fn restart(&mut self) {
        self.stop();
        self.status = Status::Restarting;
    }

    pub fn kill(&mut self) {
        if let Some(child) = &mut self.child {
            match child.kill() {
                Ok(_) => {
                    self.child = None;
                    self.status = Status::Stopped;
                }
                Err(e) => { eprintln!("{}", e) }
            }
        }
    }
    
    fn set_umask(&self, new_umask: libc::mode_t) -> mode_t {
        let old_umask = unsafe { umask(new_umask) };
        old_umask
    }

    pub fn check_process_state(&mut self, config: &Config) {
        match self.status {
            Status::Starting => {
                if self.timer.elapsed() > Duration::new(config.starttime as u64, 0) {
                    self.retries = 0;
                    self.status = Status::Running;
                    self.uptime = Instant::now();
                    println!("{}:{} is now running", self.task_name, self.id);
                }
            }
            Status::Stopping => {
                if self.timer.elapsed() > Duration::new(config.stoptime as u64, 0) {
                    self.kill();
                    println!("{}:{} is now stopped", self.task_name, self.id);
                }
            }
            Status::Restarting => {
                if self.timer.elapsed() > Duration::new(config.stoptime as u64, 0) {
                    self.kill();
                    self.start();
                }
            }
            _ => {}
        }
    }
}