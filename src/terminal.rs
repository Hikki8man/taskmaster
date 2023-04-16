use std::{io::{self, Read, stdout, Write}, sync::mpsc::Sender, process::exit, ffi::c_char};

use crate::monitor::CommandName;
use std::os::unix::io::AsRawFd;
use std::mem;
use libc::{self, c_int, tcgetattr, tcsetattr, TCSANOW, STDIN_FILENO, termios, ECHO, ICANON, ISIG, VMIN, VTIME, VINTR, VEOF, INPCK, ISTRIP, IXON};

const ENTER: char = '\n';
const BACKSPACE: char = '\x7f';
const TAB: char = '\t';
const ARROW: char = '\x1B';
const LEFT: &'static str = "[D";
const RIGHT: &'static str = "[C";
const UP: &'static str = "[A";
const DOWN: &'static str = "[B";


pub struct TermInput {
	pub cmd_name: CommandName,
	pub args: Vec<ProcessArg>
}

impl TermInput {
	pub fn new(cmd_name: CommandName, args: Vec<ProcessArg>) -> TermInput {
		TermInput { cmd_name, args }
	}
}

#[derive(Clone, Debug)]
pub struct ProcessArg {
	pub name: String,
	pub id: String,
}

pub struct Terminal {
	sender: Sender<TermInput>,
	history: Vec<String>,
}

impl Terminal {
	pub fn new(sender: Sender<TermInput>) -> Terminal {
		let stdin = io::stdin().as_raw_fd();
    	let mut orig_termios: termios = unsafe { mem::zeroed() };

		if unsafe { tcgetattr(stdin, &mut orig_termios as *mut _) } != 0 {
			panic!("tcgetattr failed");
		}
	
		let mut new_termios = orig_termios;
		new_termios.c_lflag &= !(libc::ICANON | ECHO);
		new_termios.c_iflag &= !(libc::BRKINT | INPCK | ISTRIP | IXON);
		new_termios.c_cflag |= libc::CS8;
	
		if unsafe { tcsetattr(stdin, TCSANOW, &new_termios as *const _) } != 0 {
			panic!("tcsetattr failed");
		}

		Terminal {
			sender,
			history: Vec::new(),
		}
	}

	pub fn read_input(&mut self) {
		let mut word = String::new();
		let mut saved_word:  Option<String> = None;
		let mut buf = [0; 1];
		let mut cursor_pos = 0;
		let mut index_history = 0;
		let mut tab_index = 0;
		let mut suggest_word:  Option<String> = None;
		loop {
			let n = io::stdin().read(&mut buf).unwrap();
			if n == 1 {
				let c = buf[0] as char;
				if c != TAB {
					tab_index = 0;
					suggest_word = None;
				} else if c != ARROW {
					saved_word = None;
				}
				match c {
					TAB => {
						// Tab key pressed, complete the current word
						if suggest_word.is_none() {
							suggest_word = Some(word.clone());
						}
						let suggestion = suggest_word.as_ref().map(|s| s.as_str()).unwrap_or(word.as_str());
						let completions = Self::get_completions(&suggestion);
						if completions.len() == 1 {
							// Only one completion, replace the current word with it
							word = completions[0].clone();
							Self::clear_line_and_print(&word);
							cursor_pos = word.len();
						} else if completions.len() > 1 {
							if tab_index == 0 {
								Self::clear_line();
								// Multiple completions, print them and let the user choose one
								for completion in completions {
									print!("{}		", completion);
								}
								print!("\n");
								print!("{}", word);
							} else {
								if tab_index >= completions.len() {
									tab_index = 0;
								}
								let suggestion = completions.get(tab_index).unwrap();
								Self::clear_line_and_print(suggestion);
								word = suggestion.clone();
								cursor_pos = word.len();
							}
							tab_index += 1;
						}
					}
					ENTER => {
						Self::clear_line();
						io::stdout().flush().unwrap();
						cursor_pos = 0;
						println!("{}", word);
						self.history.push(word.clone());
						index_history = self.history.len();
						Self::check_input(word.clone(), &self.sender);
						word.clear();
					}
					BACKSPACE => {
						index_history = self.history.len();
						if cursor_pos > 0 {
							cursor_pos -= 1;
							word.remove(cursor_pos);
							Self::clear_line_and_print(&word);
							// Move the cursor back to the correct position
							if cursor_pos != word.len() {
								print!("\x1b[{}D", word.len() - cursor_pos);
							}
							io::stdout().flush().unwrap();
						}
					}
					ARROW => {
						let mut buf = [0; 2];
						let n = io::stdin().read(&mut buf).unwrap();
						if n == 2 {
							let arrow = String::from_utf8(buf.to_vec()).unwrap();
							match arrow.as_str() {
								LEFT => {
									saved_word = None;
									index_history = self.history.len();
									if cursor_pos > 0 {
										cursor_pos -= 1;
										print!("\x1b[1D");
										io::stdout().flush().unwrap();
									}
								}
								RIGHT => {
									saved_word = None;
									index_history = self.history.len();
									if cursor_pos < word.len() {
										cursor_pos += 1;
										print!("\x1b[1C");
										io::stdout().flush().unwrap();
									}
								}
								UP => {
									if !self.history.is_empty() && index_history != 0 {
										let histo_at = self.history.get(index_history - 1).unwrap();
										Self::clear_line_and_print(&histo_at);
										if saved_word == None {
											saved_word = Some(word.clone());
										}
										word = histo_at.clone();
										cursor_pos = word.len();
										index_history -= 1;
										
									}

								}
								DOWN => {
									if !self.history.is_empty() {
										if index_history + 1 < self.history.len() {
											index_history += 1;
											let histo_at = self.history.get(index_history).unwrap();
											Self::clear_line_and_print(&histo_at);
											word = histo_at.clone();
											cursor_pos = word.len();
										} else if let Some(saved) = saved_word{
											Self::clear_line_and_print(&saved);
											word = saved;
											saved_word = None;
											cursor_pos = word.len();
											index_history = self.history.len();
										}
									} 
								}

								_ => {println!("{}", arrow)}
							}
						}
					}
					_ => {
						// Other key pressed, add it to the current word
						word.insert(cursor_pos, c);
						cursor_pos += 1;
						Self::clear_line_and_print(&word);
						print!("\r\x1B[{}C", cursor_pos);
					}
				}
				io::stdout().flush().unwrap();
				}
		}
	}

	fn get_completions(word: &str) -> Vec<String> {
		let commands = vec![
			String::from("status"),
			String::from("start"),
			String::from("stop"),
			String::from("shutdown"),
			String::from("restart"),
			String::from("help"),
		];
	
		commands
			.iter()
			.filter(|&s| s.starts_with(word))
			.map(|s| s.clone())
			.collect()
	}

	fn task_missing(cmd_name: &str) {
		println!("Command is missing task name. Here is an example of a command:");
		println!("{} [name of the task]", cmd_name);
	}

	fn get_task_and_arg(str: &str) -> ProcessArg {
		let args_splited: Vec<&str> = str.splitn(2, ":").collect();
		let name = String::from(args_splited[0]);
		let id = String::from(
			if let Some(id) = args_splited.get(1) {
				if id.is_empty() {
					"*"
				} else {
					id
				}
			} else {
				"*"
			});
		ProcessArg { name, id }
	}

	fn parse_args(input: &Vec<&str>) -> (Option<String>, Vec<ProcessArg>) {
		let mut i = 0;
		let mut cmd: Option<String> = None;
		let mut args: Vec<ProcessArg> = vec![];
		while i < input.len() {
			if i == 0 {
				cmd = Some(input[i].to_string());
			} else {
				args.push(Self::get_task_and_arg(input[i]));
			}
			i += 1;
		}
		(cmd, args)
	}
	
	fn check_input(input: String, sender: &Sender<TermInput>) {
		let input: Vec<&str> = input.split_whitespace().collect();
		if input.is_empty() {
			return;
		}

		let (cmd, args) = Self::parse_args(&input);
		// println!("Args: {:?}", args);
		if let Some(cmd) = cmd {
			match cmd.as_str() {
				"start" => {
					if args.is_empty() {
						return Self::task_missing(&cmd);
					}
					sender.send(TermInput::new(CommandName::START, args)).ok();
				}
				"stop" => {
					if args.is_empty() {
						return Self::task_missing(&cmd);
					}
					sender.send(TermInput::new(CommandName::STOP, args)).ok();
				}
				"restart" => {
					if args.is_empty() {
						return Self::task_missing(&cmd);
					}
					sender.send(TermInput::new(CommandName::RESTART, args)).ok();
				}
				"status" => {
					sender.send(TermInput::new(CommandName::STATUS, args)).ok();
				}
				"help" => {
					println!("Here are the command you can use:");
					println!("===================================");
					println!("start    stop    restart    status");
				}
				"shutdown" => {
					sender.send(TermInput::new(CommandName::SHUTDOWN, args)).ok();
				}
				_ => {
					println!("Command not found");
					println!("Type 'help' to see commands available");
				}

			}
		}
	}
	
	fn clear_line() {
		print!("\r\x1B[2K");
	}
	
	fn clear_line_and_print(str: &String) {
		print!("\r\x1B[2K{}", str);
	}
}
