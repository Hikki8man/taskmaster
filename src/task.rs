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

    fn get_processes_by_task_and_id(task_name: String, processes: &mut Vec<Process>, id: String) -> Vec<&mut Process> { 
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
}