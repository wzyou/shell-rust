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
            ["type", rest @ ("exit" | "echo" | "type")] => println!("{} is a shell builtin", rest),
            ["type", rest @ ..] => {
                match get_type_of_command(rest[0]) {
                    TypeCommand::Program(custom_exe) => println!("{} is {}", rest[0], custom_exe),
                    _ => println!("{}: not found", rest[0]),
                }
            }
            [commands @ ..] => {
                match get_type_of_command(commands[0]) {
                    TypeCommand::Program(custom_exe) => execute(&custom_exe, &commands[1..]),
                    _ => println!("{}: command not found", commands[0]),
                }
            },
        }
    }
}
