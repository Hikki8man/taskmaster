use std::{process::Command, error::Error};

use crate::{task_utils::Config, process::Process};

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

    fn get_processes_by_task_and_id(&mut self, processes: &mut Vec<Process>, id: String) -> Vec<&mut Process> {
        processes.iter_mut()
        .filter(|e| e.task_name == self.name)
        .filter(|e| e.id.to_string() == id || id == "*")
        .collect()
    }

    // .contain
    pub fn start(&mut self, processes: &mut Vec<Process>, id: String) {
       let procs = self.get_processes_by_task_and_id(processes, id);
        for process in procs {
            process.retries = 0;
            process.start(self);
        }
    }

    pub fn stop(&mut self, processes: &mut Vec<Process>, id: String) {
        let procs = self.get_processes_by_task_and_id(processes, id);
        for process in procs {
            process.stop(self);
        }
    }

    pub fn restart(&mut self, processes: &mut Vec<Process>, id: String) {
        let procs = self.get_processes_by_task_and_id(processes, id);
        for process in procs {
            process.retries = 0;
            process.restart(self);
        }
    }
}