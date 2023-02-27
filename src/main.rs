use std::{fs::File, process::exit};
use std::io::Read;
use std::env;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    cmd: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        3.. => {
            println!("Too many arguments. Useage: ./executable path_to_config");
            exit(-1);
        },
        2 => {
            println!("Checking provided path to configuration file...");
        },
        _ => {
            println!("No arguments given. Checking default path to configuration file...");
        }
    }



    let mut file = File::open("tasks.yaml")
        .expect("Could not open file...");
    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect("Could not read file...");
    
    let tasks: std::collections::HashMap<String, Task> =
        serde_yaml::from_str(content.as_str()).unwrap();

    for(name, task) in tasks {
        println!("App: {0}", name);
        println!("\tStart Command: {0}", task.cmd);
    }
}
