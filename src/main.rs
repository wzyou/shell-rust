#[allow(unused_imports)]
use std::io::{self, Write};
use std::{env, fs};

fn get_path() -> Vec<String> {
    env::var("PATH")
        .unwrap_or("".to_string())
        .split(':')
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
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
            ["type", rest @ ..] => 'path: {
                let paths = get_path();
                for path in &paths {
                    if let Ok(entrues) = fs::read_dir(path) {
                        for _dir in entrues {
                            if let Ok(dir) = _dir {
                                if dir.file_name() == rest[0] {
                                    println!("{} is {}/{}", rest[0], path, rest[0]);
                                    break 'path;
                                }
                            }
                        }
                    }
                }
                println!("{}: not found", rest[0])
            }
            _ => println!("{}: command not found", full_command[0]),
        }
    }
}
