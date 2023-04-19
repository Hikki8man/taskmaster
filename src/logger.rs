use std::{fs::File, io::Write};

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

    pub fn write(&mut self, buf: &str) {
        if let Some(log) = &mut self.log_file {
            log.write_all(buf.as_bytes());

        }
    }
}