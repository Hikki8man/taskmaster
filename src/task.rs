use std::{vec};

use crate::{task_utils::{Config, Autorestart}, process::{Process, Status}, print_process};

#[derive(Debug)]
pub struct Task {
    pub name: String,
    pub processes: Vec<Process>,
    pub config: Config,
}

impl Task {
    pub fn new(config: Config, name: String) -> Task {
        Task { config, name, processes: vec![] }
    }

    fn get_procs_by_id(&mut self, id: String) -> Vec<&mut Process> {
        let id_clone = id.clone();
        let procs: Vec<&mut Process> = self.processes.iter_mut().filter(move |e| e.id.to_string() == id || matches!(id.as_str(), "*")).collect();
        match procs.is_empty() {
            true if id_clone == "*" => eprintln!("No processes found for task {}", self.name),
            true =>  eprintln!("Process {}:{} not found", self.name, id_clone),
            false => {},
        }
        procs
    }
    
    pub fn start(&mut self, id: String) {
        let procs = self.get_procs_by_id(id);
        for process in procs {
            process.retries = 0;
            process.start();
        }
    }

    pub fn stop(&mut self, id: String) {
        let procs = self.get_procs_by_id(id);
        for process in procs {
            process.stop();
        }
    }

    pub fn restart(&mut self, id: String) {
        let procs = self.get_procs_by_id(id);
        for process in procs {
            process.retries = 0;
            process.restart();
        }
    }

    pub fn print_processes(&mut self, id: String) {
	    let procs: Vec<&mut Process> = self.processes.iter_mut().filter(|e| e.id.to_string() == id || id == "*").collect();
        match procs.is_empty() {
            true if id == "*" => eprintln!("No processes found for task {}", self.name),
            true =>  eprintln!("Process {}:{} not found", self.name, id),
            false => {},
        }
        for proc in procs {
            let status = match proc.status {
                Status::Running => "\x1B[32mRunning\x1B[0m",
                Status::Stopping => "\x1B[31mStopping\x1B[0m",
                Status::Stopped => "\x1b[30mStopped\x1B[0m",
                Status::Restarting => "\x1B[33mRestarting\x1B[0m",
                Status::Fatal => "\x1B[31mFatal\x1B[0m",
                _ => "\x1B[33mStarting\x1B[0m",
            };
            let format = if self.config.numprocs > 1 { format!("{}:{}", self.name, proc.id) }
                else { self.name.clone() };
            if let Some(err) = &proc.error {
                print_process!(format, status, err);
            }
            else if let Some(child) = &proc.child {
                let uptime = proc.uptime.elapsed();
                let uptime_formatted = format!(
                    "{:02}:{:02}:{:02}", 
                    uptime.as_secs() / 3600, 
                    (uptime.as_secs() / 60) % 60, 
                    uptime.as_secs() % 60
                );
                if proc.status == Status::Running {
                    print_process!(format, status, child.id(), uptime_formatted);
                } else {
                    print_process!(format, status, child.id());
                }
            } else {
                print_process!(format, status);
            }

        }
	}

    pub fn try_wait(&mut self) {
        for process in self.processes.iter_mut() {
            if let Some(child) = &mut process.child {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        println!("exit status: {:?}, Process status: {:?}", status.code(), process.status);
                        process.child = None;
                        match process.status {
                            Status::Starting => {
                                if process.retries < self.config.startretries {
                                    process.start();
                                } else {
                                    process.status = Status::Fatal;
                                }
                            }
                            Status::Stopping => { process.status = Status::Stopped }
                            Status::Restarting => {	process.start(); }
                            _ => {
                                match self.config.autorestart {
                                    Autorestart::Always => {
                                        process.start();
                                    }
                                    Autorestart::Unexpected => {
                                        if status.code().is_none() || !self.config.exitcodes.contains(&status.code().unwrap_or(0)) {
                                            process.start();
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
                        process.check_process_state(&self.config);
                    }
                    Err(e) => println!("error attempting to wait: {}", e),
                }
            }
        }
    }

    pub fn wait_procs_to_stop(&mut self) {
        loop {
            self.try_wait();
            if !self.processes.iter().any(|p| {
				p.status != Status::Stopped && p.status != Status::Fatal
			}) {
                break;
            }
        }
    }
}