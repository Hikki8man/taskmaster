use std::path::PathBuf;
use std::{fs::File, process::exit};
use std::io::Read;
use std::env;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    cmd: String,
}

macro_rules! print_exit {
    ($err_msg:expr, $err_code:expr) => {
        println!("{}", $err_msg);
        exit($err_code);
    };
}

fn main() {
    let default_path = PathBuf::from("tasks.yaml");
    let args: Vec<String> = env::args().collect();

    match args.len() {
        3.. => {
            println!("Too many arguments. Useage: ./executable [path_to_config]");
            exit(-1);
        },
        _ => {
            println!("Checking path to configuration file...");
        }
    }

    let path = env::args()
                    .nth(1)
                    .map(PathBuf::from)
                    .unwrap_or(default_path);
    println!("{:?}", path);
    let extension = path.extension();
    if extension.is_none() || extension.unwrap() != "yaml"
    {
        print_exit!("Wrong file extention. Expecting a YAML file.", -1);
    }
    if !path.try_exists().expect("Unable to check file existence.")
    {
        print_exit!("Invalid path.", -1);
    }
    if !path.is_file()
    {
        print_exit!("Not a file.", -1);
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
