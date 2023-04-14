use std::{process::Command, error::Error};

use crate::{task_utils::Config, process::{Process, Status}, terminal::ProcessArg, print_process};

#[derive(Debug)]
pub struct Task {
    pub name: String,
    pub config: Config,
    pub cmd: Command,
    pub error: Option<Box<dyn Error>>,
}

impl Task {
    pub fn new(config: Config, cmd: Command, name: String) -> Task {
        Task { config, cmd, name, error: None }
    }

    pub fn get_processes_by_task_and_id(task_name: String, processes: &mut Vec<Process>, id: String) -> Vec<&mut Process> { 
        let procs: Vec<&mut Process> = if id == "*" {
            processes.iter_mut()
                .filter(|e| e.task_name == task_name)
                .collect()
        } else {
            processes.iter_mut()
                .filter(|e| e.task_name == task_name && e.id.to_string() == id)
                .collect()
        };
        match procs.is_empty() {
            true if id == "*" => eprintln!("No processes found for task {}", task_name),
            true => eprintln!("Process {}:{} not found", task_name, id),
            false => {},
        }
        procs
    }
    
    pub fn start(&mut self, processes: &mut Vec<Process>, id: String) {
       let procs = Self::get_processes_by_task_and_id(self.name.to_string(), processes, id.to_string());
        for process in procs {
            process.retries = 0;
            process.start(self);
        }
    }

    pub fn stop(&mut self, processes: &mut Vec<Process>, id: String) {
        let procs = Self::get_processes_by_task_and_id(self.name.to_string(), processes, id);
        for process in procs {
            process.stop(self);
        }
    }

    pub fn restart(&mut self, processes: &mut Vec<Process>, id: String) {
        let procs = Self::get_processes_by_task_and_id(self.name.to_string(), processes, id);
        for process in procs {
            process.retries = 0;
            process.restart(self);
        }
    }

    pub fn print_processes(&mut self, processes: &mut Vec<Process>, id: String) {
	    let procs = Task::get_processes_by_task_and_id(self.name.to_string(), processes, id);
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
            if let Some(err) = &self.error {
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
}