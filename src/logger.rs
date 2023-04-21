use std::{fs::File, io::Write};
use chrono::{Local};

pub struct Logger {
    log_file: Option<File>,
}

impl Logger {
    pub fn new() -> Logger {
        let log_file = match File::create("task.log") {
            Ok(file) => { Some(file) },
            Err(e) => {
                eprint!("Logger failed to create: {}", e);
                None
            }
        };
        Logger { log_file }
    }

    pub fn write(&mut self, buf: String) {
        if let Some(log) = &mut self.log_file {
            let date = Local::now().format("%Y-%m-%d %H:%M:%S,%3f ").to_string();
            log.write_all((date + &buf + "\n").as_bytes()).ok();

        }
    }
}