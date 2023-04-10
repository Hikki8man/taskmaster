use std::{io::{self, Read, stdout, Write}, sync::mpsc::Sender, process::exit, ffi::c_char};

use crate::monitor::CommandName;

const ENTER: char = '\n';
const BACKSPACE: char = '\x7f';
const TAB: char = '\t';
const ARROW: char = '\x1B';
const LEFT: &'static str = "[D";
const RIGHT: &'static str = "[C";
const UP: &'static str = "[A";
const DOWN: &'static str = "[B";

#[derive(Clone, Debug)]
pub struct TermInput {
	pub name: CommandName,
	pub arg: String,
}

fn task_missing(cmd_name: &str) {
	println!("Command is missing task name. Here is an example of a command:");
	println!("{} [name of the task]", cmd_name);
}

fn check_input(input: String, sender: &Sender<TermInput>) {
	let input_vec: Vec<&str> = input.split_whitespace().collect();
	if input_vec.is_empty() {
		return;
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
			println!("start    stop    restart    status");
		}
		"shutdown" => {
			//TODO stop all process
			exit(1);
		}
		_ => {
			println!("Command not found");
			println!("Type 'help' to see commands available");
		}
	}
}



use std::os::unix::io::AsRawFd;
use std::mem;
use libc::{self, c_int, tcgetattr, tcsetattr, TCSANOW, STDIN_FILENO, termios, ECHO, ICANON, ISIG, VMIN, VTIME, VINTR, VEOF, INPCK, ISTRIP, IXON};

fn clear_line() {
	print!("\r\x1B[2K");
}

fn clear_line_and_print(str: &String) {
	print!("\r\x1B[2K{}", str);
}

pub fn read_input(sender: Sender<TermInput>) {
    let stdin = io::stdin().as_raw_fd();
	let mut history: Vec<String> = Vec::new();
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
					let completions = get_completions(&suggestion);
					if completions.len() == 1 {
						// Only one completion, replace the current word with it
						word = completions[0].clone();
						clear_line_and_print(&word);
						cursor_pos = word.len();
					} else if completions.len() > 1 {
						if tab_index == 0 {
							clear_line();
							// Multiple completions, print them and let the user choose one
							println!("Possible completions:");
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
							clear_line_and_print(suggestion);
							word = suggestion.clone();
							cursor_pos = word.len();
						}
						tab_index += 1;
					}
           		}
       			ENTER => {
					clear_line();
					io::stdout().flush().unwrap();
					cursor_pos = 0;
					println!("{}", word);
					history.push(word.clone());
					index_history = history.len();
					check_input(word.clone(), &sender);
       			    word.clear();
       			}
				BACKSPACE => {
					index_history = history.len();
					if cursor_pos > 0 {
						cursor_pos -= 1;
						word.remove(cursor_pos);
						clear_line_and_print(&word);
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
								index_history = history.len();
								if cursor_pos > 0 {
									cursor_pos -= 1;
									print!("\x1b[1D");
									io::stdout().flush().unwrap();
								}
							}
							RIGHT => {
								saved_word = None;
								index_history = history.len();
								if cursor_pos < word.len() {
									cursor_pos += 1;
									print!("\x1b[1C");
									io::stdout().flush().unwrap();
								}
							}
							UP => {
								if !history.is_empty() && index_history != 0 {
									let histo_at = history.get(index_history - 1).unwrap();
									clear_line_and_print(&histo_at);
									if saved_word == None {
										saved_word = Some(word.clone());
									}
									word = histo_at.clone();
									cursor_pos = word.len();
									index_history -= 1;
									
								}

							}
							DOWN => {
								if !history.is_empty() {
									if index_history + 1 < history.len() {
										index_history += 1;
										let histo_at = history.get(index_history).unwrap();
										clear_line_and_print(&histo_at);
										word = histo_at.clone();
										cursor_pos = word.len();
									} else if let Some(saved) = saved_word{
										clear_line_and_print(&saved);
										word = saved;
										saved_word = None;
										cursor_pos = word.len();
										index_history = history.len();
									}
								} 
							}

							_ => {println!("{}", arrow)}
						}
					}
				}
       			_ => {
       			  	// Other key pressed, add it to the current word
					// println!("cursor pos: {}", cursor_pos);
					word.insert(cursor_pos, c);
					cursor_pos += 1;
					clear_line_and_print(&word);
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




