use std::{io::{self, Read, stdout, Write}, sync::mpsc::Sender, process::exit, ffi::c_char};

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



use std::os::unix::io::AsRawFd;
use std::mem;
use libc::{self, c_int, tcgetattr, tcsetattr, TCSANOW, STDIN_FILENO, termios, ECHO, ICANON, ISIG, VMIN, VTIME, VINTR, VEOF};


pub fn read_input(sender: Sender<TermInput>) {
    let stdin = io::stdin().as_raw_fd();
	let mut history: Vec<String> = Vec::new();
    let mut orig_termios: termios = unsafe { mem::zeroed() };

    if unsafe { tcgetattr(stdin, &mut orig_termios as *mut _) } != 0 {
        panic!("tcgetattr failed");
    }

    let mut new_termios = orig_termios;
    new_termios.c_lflag &= !(libc::ICANON  | libc::ECHO /* | libc::ISIG */);
    new_termios.c_iflag &= !(libc::BRKINT | libc::INPCK | libc::ISTRIP | libc::IXON);
    new_termios.c_cflag |= libc::CS8;

    if unsafe { tcsetattr(stdin, TCSANOW, &new_termios as *const _) } != 0 {
        panic!("tcsetattr failed");
    }

    let mut word = String::new();
	let mut saved_word:  Option<String> = None;
	let mut buf = [0; 1];
	let mut cursor_pos = 0;
	let mut index_history = 0;

	loop {
    	let n = io::stdin().read(&mut buf).unwrap();
    	if n == 1 {
        	let c = buf[0] as char;
        	match c {
            	'\t' => {
					saved_word = None;
					io::stdout().flush().unwrap();
        			// Tab key pressed, complete the current word
        			let completions = get_completions(&word);
        			if completions.len() == 1 {
        			    // Only one completion, replace the current word with it
        			    word = completions[0].clone();
						print!("\r\x1B[2K"); // Move the cursor to the beginning of the line and clear to the end of the line
   					 	io::stdout().flush().unwrap();
        			    print!("{}", word);
						cursor_pos = word.len();
        			} else if completions.len() > 1 {
						print!("\r\x1B[2K");
        			    // Multiple completions, print them and let the user choose one
        			    println!("Possible completions:");
        			    for completion in completions {
        			        print!("{}		", completion);
        			    }
						print!("\n");
						print!("{}", word);
        			}
           		}
       			'\n' => {
					saved_word = None;
					print!("\r\x1B[2K");
					io::stdout().flush().unwrap();
					cursor_pos = 0;
					println!("{}", word);
					history.push(word.clone());
					index_history = history.len();
					check_input(word.clone(), &sender);
       			    word.clear();
       			}
       			'\r' => {}
				'\x7f' => {
					saved_word = None;
					index_history = history.len();
					if cursor_pos > 0 {
						cursor_pos -= 1;
						word.remove(cursor_pos);
						// Erase the entire line and reprint the modified word
						print!("\r\x1B[2K{}", word);
						// Move the cursor back to the correct position
						if cursor_pos != word.len() {
							print!("\x1b[{}D", word.len() - cursor_pos);
						}
						io::stdout().flush().unwrap();
					}
				}
				'\x1B' => {
					let mut buf = [0; 2];
					let n = io::stdin().read(&mut buf).unwrap();
    				if n == 2 {
						let arrow = String::from_utf8(buf.to_vec()).unwrap();
						match arrow.as_str() {
							"[D" => {
								//left
								saved_word = None;
								index_history = history.len();
								if cursor_pos > 0 {
									cursor_pos -= 1;
									print!("\x1b[1D");
									io::stdout().flush().unwrap();
								}
							}
							"[C" => {
								//right
								saved_word = None;
								index_history = history.len();
								if cursor_pos < word.len() {
									cursor_pos += 1;
									print!("\x1b[1C");
									io::stdout().flush().unwrap();
								}
							}
							"[A" => { // TODO save current word
								//up
								// println!("index: {}", index_history);
								if !history.is_empty() && index_history != 0 {
									let histo_at = history.get(index_history - 1).unwrap();
									print!("\r\x1B[2K{}", histo_at);
									if saved_word == None {
										saved_word = Some(word.clone());
									}
									word = histo_at.clone();
									cursor_pos = word.len();
									index_history -= 1;
									
								}

							}
							"[B" => {
								//down
								if !history.is_empty() {
									if index_history + 1 < history.len() {
										index_history += 1;
										let histo_at = history.get(index_history).unwrap();
										print!("\r\x1B[2K{}", histo_at);
										word = histo_at.clone();
										cursor_pos = word.len();
									} else if let Some(saved) = saved_word{
										print!("\r\x1B[2K{}", saved);
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
					saved_word = None;
					word.insert(cursor_pos, c);
					cursor_pos += 1;
					print!("\r\x1B[2K");
					print!("{}", word);
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




