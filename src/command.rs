use std::{sync::mpsc::Sender, time::Duration, thread, process::{Command, self}, fmt::Debug};

use crate::{CmdInput, Task, Message, CommandName, Status, StatusAndId};

pub fn execute_cmd(mut cmd: CmdInput, task: &mut Task, sender: Sender<Message>) {
    match cmd.name {
        CommandName::START => {
            // let numprocs: usize = task.config.numprocs as usize;
            // if task.processes.len() == numprocs {
            //     println!("{} already running", cmd.arg);
            //     return;
            // }
            for process in &mut task.processes {
                println!("retries: {}", process.retries);
                if process.child.is_some() || process.retries >= task.config.startretries {
                    continue;
                }
                let res = task.cmd.spawn();
                match res {
                    Ok(child) => {
                        let cmd_clone = cmd.clone();
                        // println!("ok: {:?}", child);
                        let sender_clone = sender.clone();
                        let id = process.id;
                        let status_update = Some(StatusAndId { status: Status::Starting, id});
                        sender.send(Message { cmd_input: cmd_clone.clone(), status_update }).expect("msg");
                        process.child = Some(child);
                        let start_time = task.config.starttime.into();
                        thread::spawn(move || {
                            thread::sleep(Duration::from_secs(start_time));
                            let status_update = Some(StatusAndId { status: Status::Running, id });
                            sender_clone.send(Message { cmd_input: cmd_clone, status_update }).expect("msg");
                        });
                        // task.processes.push(child);
                    },
                    Err(err) => {
                        println!("err: {:?}", err);
                        process.retries += 1;
                        cmd.from_term = false;
                        sender.send(Message { cmd_input: cmd.clone(), status_update: None }).expect("msg");
                    }
                }
            }
            // while task.processes.len() < numprocs /*&& task.processes < task.config.startretries*/ {
            //     let res = task.cmd.spawn();
            //     match res {
            //         Ok(child) => {
            //             println!("ok: {:?}", child);
            //             task.processes.push(child);
            //         },
            //         Err(err) => {
            //             println!("err: {:?}", err);
            //             task.retries += 1;
            //         }
            //     }
            // }
            // println!("error");
        }
        CommandName::STOP => {
            // if task.child.is_empty() {
            //     println!("{} is not running", cmd.arg);
            //     return;
            // }
            let mut kill = Command::new("kill");
            for process in &mut task.processes {
                if let Some(child) = &process.child {
                    kill.args(["-s", "TERM", child.id().to_string().as_str()]);
                    let mut pid = kill.spawn().expect("msg");
                    pid.wait().expect("msg");
                    let cmd_clone = cmd.clone();
                    let id = process.id;
                    process.child = None;
                    let status_update = Some(StatusAndId { status: Status::Stopping, id });
                    let sender_clone = sender.clone();
                    sender.send(Message { cmd_input: cmd_clone.clone(), status_update }).expect("msg");
                    // println!("gello");

                    let stop_time = task.config.stoptime.into();
                    thread::spawn(move || {
                        thread::sleep(Duration::from_secs(stop_time));
                        let status_update = Some(StatusAndId { status: Status::Stopped, id });
                        sender_clone.send(Message { cmd_input: cmd_clone, status_update }).expect("msg");
                    });
                }
            }
            // sender.send(Message { cmd_input: cmd_clone, status_update: Some(Status::Stopping) }).expect("msg");
        }
        CommandName::RESTART => {
            // let cmd_clone = cmd.clone();

            // sender.send(Message { cmd_input: cmd_clone, status_update: Some(Status::Restarting) }).expect("msg");
            // if !task.child.is_empty() {
            //     let mut kill = Command::new("kill");
            //     let mut i = 0;
            //     task.status = Status::Starting;
            //     while i < task.child.len() {
            //         kill.args(["-s", "TERM", task.child[i].id().to_string().as_str()]);
            //         let mut pid = kill.spawn().expect("msg");
            //         pid.wait().expect("msg");
            //         task.child.remove(i);
            //         i += 1;
            //     }
            // }
            // let numprocs: usize = task.config.numprocs as usize;

            // while task.child.len() < numprocs {
            //     task.child.push(task.cmd.spawn().expect("msg"));
            // }
            // let restart_time = task.config.stoptime + task.config.starttime;
            // thread::spawn(move || {
            //     thread::sleep(Duration::from_secs(restart_time.into()));
            //     sender.send(Message { cmd_input: cmd, status_update: Some(Status::Running) }).expect("msg");
            // });
        }
        _ => {}
    }
}