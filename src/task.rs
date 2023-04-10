use std::process::Command;

use crate::{task_utils::Config, process::Process};

#[derive(Debug)]
pub struct Task {
    pub name: String,
    pub config: Config,
    pub cmd: Command,
}

impl Task {
    pub fn new(config: Config, cmd: Command, name: String) -> Task {
        Task { config, cmd, name }
    }

    pub fn start(&mut self, processes: &mut Vec<Process>) {
        for process in processes {
            if process.task_name == self.name {
                process.retries = 0;
                process.start(self);
            }
        }
    }

    pub fn stop(&mut self, processes: &mut Vec<Process>) {
        for process in processes {
            if process.task_name == self.name {
                process.stop(self);
            }
        }
    }

    pub fn restart(&mut self, processes: &mut Vec<Process>) {
        for process in processes {
            if process.task_name == self.name {
                process.retries = 0;
                process.restart(self);
            }
        }
    }
}