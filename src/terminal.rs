use std::{io, sync::mpsc::Sender};

use crate::{CmdInput, CommandName, Message};

pub fn read_input(sender: Sender<Message>) {
	loop {
		let mut buffer = String::new();
		io::stdin().read_line(&mut buffer).expect("msg");
		let input_vec: Vec<&str> = buffer.split_whitespace().collect();
		if input_vec.is_empty() {
			continue;
		}
		match input_vec[0] {
			"start" => {
				if input_vec.len() > 1 {
					let cmd: CmdInput = CmdInput { name: CommandName::START, arg: String::from(input_vec[1]) };
					let msg = Message { cmd_input: cmd, status_update: None };
					sender.send(msg).expect("msg");
				}
			}
			"stop" => {
				if input_vec.len() > 1 {
					let cmd: CmdInput = CmdInput { name: CommandName::STOP, arg: String::from(input_vec[1]) };
					let msg = Message { cmd_input: cmd, status_update: None };
					sender.send(msg).expect("msg");
				}
			}
			"restart" => {
				if input_vec.len() > 1 {
					let cmd: CmdInput = CmdInput { name: CommandName::RESTART, arg: String::from(input_vec[1]) };
					let msg = Message { cmd_input: cmd, status_update: None };
					sender.send(msg).expect("msg");
				}
			}
			"status" => {
				let cmd: CmdInput = CmdInput { name: CommandName::STATUS, arg: String::from(input_vec[0]) };
					let msg = Message { cmd_input: cmd, status_update: None };
					sender.send(msg).expect("msg");
			}
			"exit" => {
				break;
			}
			_ => {
				println!("Command not found");
			}
		}
	}
}