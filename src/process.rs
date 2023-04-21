use std::{process::{Child, Command}, time::{Instant, Duration}, error::Error, io::{self, Write}};
use libc::{self, mode_t, umask};
use crate::{task_utils::{Sigtype, Config}, logger::Logger};

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
    stop_sig: Sigtype,
    pub task_name: String,
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

    pub fn start(&mut self, logger: &mut Logger) {
        if let Some(_child) = &self.child {
            return println!("Process {}:{} is already running", self.task_name, self.id);
        }
        if self.error.is_none() {
            let old_umask = self.set_umask(self.umask);
            match self.cmd.spawn() {
                Ok(child) => {
                    self.status = Status::Starting;
                    self.timer = Instant::now();
                    logger.write(format!("INFO spawned: '{}:{}' with pid {}", self.task_name, self.id, child.id()));
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

    pub fn stop(&mut self, logger: &mut Logger) {
        // if self.status != Status::Running { return; }
        let sig_code = self.stop_sig as u32;
        let sig_format = format!("-{}", sig_code);
        if let Some(child) = &self.child {
            let mut kill_cmd = Command::new("kill");
            match kill_cmd.args([&sig_format, child.id().to_string().as_str()]).output() {
                Ok(_) => {
                    logger.write(format!("INFO waiting for '{}:{}' to stop", self.task_name, self.id));
                    self.timer = Instant::now();
                    self.status = Status::Stopping;
                }
                Err(e) => { eprintln!("{}", e) }
            }
        }
    }

    pub fn restart(&mut self, logger: &mut Logger) {
        match self.status {
            Status::Stopped => self.start(logger),
            _ => {
                self.stop(logger);
                self.status = Status::Restarting;
            }
        }
    }

    pub fn kill(&mut self, logger: &mut Logger) {
        if let Some(child) = &mut self.child {
            logger.write(format!("Warn killing '{}:{}' ({}) with SIGKILL", self.task_name, self.id, child.id()));
            match child.kill() {
                Ok(_) => {
                    logger.write(format!("INFO exited: '{}:{}' (terminated by SIGKILL)", self.task_name, self.id));
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

    pub fn check_process_state(&mut self, config: &Config, logger: &mut Logger) {
        match self.status {
            Status::Starting => {
                if self.timer.elapsed() > Duration::new(config.starttime as u64, 0) {
                    self.retries = 0;
                    self.status = Status::Running;
                    self.uptime = Instant::now();
                    logger.write(format!("INFO success: '{}:{}' is now in a running state", self.task_name, self.id));
                    println!("{}:{} is now running", self.task_name, self.id);
                }
            }
            Status::Stopping => {
                if self.timer.elapsed() > Duration::new(config.stoptime as u64, 0) {
                    self.kill(logger);
                    println!("{}:{} is now stopped", self.task_name, self.id);
                }
            }
            Status::Restarting => {
                if self.timer.elapsed() > Duration::new(config.stoptime as u64, 0) {
                    self.kill(logger);
                    self.start(logger);
                }
            }
            _ => {}
        }
    }
}