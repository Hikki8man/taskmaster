use std::{sync::mpsc::Sender, time::Duration, thread, process::Command};

use crate::{CmdInput, Process, Message, CommandName, Status};

pub fn execute_cmd(cmd: CmdInput, process: &mut Process, sender: Sender<Message>) {
    match cmd.name {
        CommandName::START => {
            let numprocs: usize = process.task.numprocs as usize;
            if process.child.len() == numprocs {
                println!("{} already running", cmd.arg);
                return;
            }
            while process.child.len() < numprocs {
                process.child.push(process.cmd.spawn().expect("msg"));
            }
            let start_time = process.task.starttime.into();
            let cmd_clone = cmd.clone();
            sender.send(Message { cmd_input: cmd_clone, status_update: Some(Status::Starting) }).expect("msg");
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(start_time));
                sender.send(Message { cmd_input: cmd, status_update: Some(Status::Running) }).expect("msg");
            });
        }
        CommandName::STOP => {
            if process.child.is_empty() {
                println!("{} is not running", cmd.arg);
                return;
            }
            let mut i = 0;
            let mut kill = Command::new("kill");
            process.status = Status::Starting;
            while i < process.child.len() {
                kill.args(["-s", "TERM", process.child[i].id().to_string().as_str()]);
                let mut pid = kill.spawn().expect("msg");
                pid.wait().expect("msg");
                process.child.remove(i);
                i += 1;
            }
            let cmd_clone = cmd.clone();
            sender.send(Message { cmd_input: cmd_clone, status_update: Some(Status::Stopping) }).expect("msg");
            let stop_time = process.task.stoptime.into();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(stop_time));
                sender.send(Message { cmd_input: cmd, status_update: Some(Status::Stopped) }).expect("msg");
            });
        }
        CommandName::RESTART => {
            let cmd_clone = cmd.clone();

            sender.send(Message { cmd_input: cmd_clone, status_update: Some(Status::Restarting) }).expect("msg");
            if !process.child.is_empty() {
                let mut kill = Command::new("kill");
                let mut i = 0;
                process.status = Status::Starting;
                while i < process.child.len() {
                    kill.args(["-s", "TERM", process.child[i].id().to_string().as_str()]);
                    let mut pid = kill.spawn().expect("msg");
                    pid.wait().expect("msg");
                    process.child.remove(i);
                    i += 1;
                }
            }
            let numprocs: usize = process.task.numprocs as usize;

            while process.child.len() < numprocs {
                process.child.push(process.cmd.spawn().expect("msg"));
            }
            let restart_time = process.task.stoptime + process.task.starttime;
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(restart_time.into()));
                sender.send(Message { cmd_input: cmd, status_update: Some(Status::Running) }).expect("msg");
            });
        }
        _ => {}
    }
}