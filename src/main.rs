#[allow(unused_imports)]
use std::io::{self, Write};
use std::{env, fs, os::unix::fs::PermissionsExt, process::Command};

fn get_path() -> Vec<String> {
    env::var("PATH")
        .unwrap_or("".to_string())
        .split(':')
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
}

fn get_type_of_command(command: &str) -> TypeCommand {
    let paths = get_path();
    for path in &paths {
        let Ok(entrues) = fs::read_dir(path) else { continue; };
        for _dir in entrues {
            let Ok(dir) = _dir else { continue; };
            if let (Some(file_name), Ok(metadata)) = (dir.file_name().to_str(), dir.metadata()) {
                if file_name == command && metadata.permissions().mode() & 0o111 != 0 {
                    // println!("{} is {}/{}", file_name, path, file_name);
                    // break 'search;
                    return TypeCommand::Program(format!("{}/{}", path, file_name));
                }
            }
        }
        
    }
    TypeCommand::None
}

enum TypeCommand {
    // Builtin(String),
    Program(String),
    None,
}

fn execute(custom_exe: &str, args: &[&str]) {
    match Command::new(custom_exe)
        .args(args.iter())
        .status() {
        Ok(_) => {},
        Err(e) => eprintln!("failed to spawn: {}", e),
    }
}

mod builtin {
    use std::{env, path::Path};

    pub fn pwd() -> String {
        env::var("PWD").unwrap_or_else(|_| "/".to_string())
    }

    pub fn cd(dir: &str) {
        let current_dir: String;
        if dir.starts_with("/") {
            current_dir = String::new();
        } else {
            current_dir = pwd();
        }

        let target_path = current_dir.to_string() + "/" + dir;
        match Path::new(&target_path).try_exists() {
            Ok(true) => {},
            _ => {
                println!("cd: {}: No such file or directory", dir);
                return;
            },
        }

        let mut current_paths:Vec<&str> = current_dir.split("/").collect();
        // let target_paths = &current_paths[..];
        let paths:Vec<&str> = dir.split("/").collect();
        for path in paths {
            match path {
                ".." => { current_paths.pop(); },
                "." => {},
                p => { current_paths.push(p); },
            }
        }

        let target:String = current_paths.join("/");
        unsafe { env::set_var("PWD", target) };
    }
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();

        let full_command: Vec<&str> = command.split_whitespace().collect();

        match full_command.as_slice() {
            [] => continue,
            ["exit"] => break,
            ["echo", rest @ ..] => println!("{}", rest.join(" ")),
            ["pwd"] => println!("{}", builtin::pwd()),
            ["cd", dir] => builtin::cd(*dir),
            ["type", rest @ ("exit" | "echo" | "type" | "pwd")] => println!("{} is a shell builtin", rest),
            ["type", rest @ ..] => {
                match get_type_of_command(rest[0]) {
                    TypeCommand::Program(custom_exe) => println!("{} is {}", rest[0], custom_exe),
                    _ => println!("{}: not found", rest[0]),
                }
            }
            [commands @ ..] => {
                match get_type_of_command(commands[0]) {
                    TypeCommand::Program(_) => execute(commands[0], &commands[1..]),
                    _ => println!("{}: command not found", commands[0]),
                }
            },
        }
    }
}
