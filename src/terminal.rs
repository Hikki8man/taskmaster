use std::{io::{self}, sync::mpsc::Sender, process::exit};

use crate::monitor::CommandName;

#[derive(Clone, Debug)]
pub struct TermInput {
	pub name: CommandName,
	pub arg: String,
}


fn task_missing(cmd_name: &str) {
	println!("Command is missing task name. Here is an example of a command:");
	println!("{} [name of the task]\n", cmd_name);
}

pub fn read_input(sender: Sender<TermInput>) {
	loop {
		let mut buffer = String::new();
		// print!("> ");
        // io::stdout().flush().unwrap();
		io::stdin().read_line(&mut buffer).expect("msg");
		let input_vec: Vec<&str> = buffer.split_whitespace().collect();
		if input_vec.is_empty() {
			continue;
		}
		match input_vec[0] {
			"start" => {
				if input_vec.len() > 1 {
					let msg: TermInput = TermInput { name: CommandName::START, arg: String::from(input_vec[1]) };
					sender.send(msg).expect("msg");
				} else {
					task_missing(input_vec[0])
				}
			}
			"stop" => {
				if input_vec.len() > 1 {
					let msg: TermInput = TermInput { name: CommandName::STOP, arg: String::from(input_vec[1]) };
					sender.send(msg).expect("msg");
				} else {
					task_missing(input_vec[0])
				}
			}
			"restart" => {
				if input_vec.len() > 1 {
					let msg: TermInput = TermInput { name: CommandName::RESTART, arg: String::from(input_vec[1]) };
					sender.send(msg).expect("msg");
				} else {
					task_missing(input_vec[0])
				}
			}
			"status" => {
				let msg: TermInput = TermInput { name: CommandName::STATUS, arg: String::from("") };
				sender.send(msg).expect("msg");
			}
			"help" => {
				println!("Here are the command you can use:");
				println!("===================================");
				println!("start		stop 	restart 	status\n");
			}
			"shutdown" => {
				//TODO stop all process
				exit(1);
			}
			_ => {
				println!("Command not found");
				println!("Type 'help' to see commands available\n");
			}
		}
	}
}