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
        let Ok(entrues) = fs::read_dir(path) else {
            continue;
        };
        for _dir in entrues {
            let Ok(dir) = _dir else {
                continue;
            };
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
    match Command::new(custom_exe).args(args.iter()).status() {
        Ok(_) => {}
        Err(e) => eprintln!("failed to spawn: {}", e),
    }
}

mod builtin {
    use anyhow::{Context, Result};
    use std::{env, path::PathBuf};

    pub fn pwd() -> PathBuf {
        env::current_dir().unwrap()
    }

    pub fn cd(dir: &str) -> Result<()> {
        let cwd = pwd();
        let dest; //= cwd.join(dir);
        match dir {
            "~" => dest = cwd.join(env::var("HOME")?),
            "-" => dest = cwd.join(env::var("OLDPWD")?),
            _ => dest = cwd.join(dir),
        }

        env::set_current_dir(&dest)
            .with_context(|| format!("{}: No such file or directory", dest.display()))?;

        unsafe {
            env::set_var("OLDPWD", &cwd);
        }
        let new_pwd = env::current_dir()?;
        unsafe {
            env::set_var("PWD", &new_pwd);
        }

        Ok(())
    }
}

fn parse_shell_args(intput: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut arg = String::new();
    let mut is_single_quotes = false;
    let mut is_double_quotes = false;

    let mut chars = intput.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' if !is_double_quotes => is_single_quotes = !is_single_quotes,
            '"' if !is_single_quotes => is_double_quotes = !is_double_quotes,
            '\\' if !is_single_quotes => {
                if let Some(next_c) = chars.peek() {
                    arg.push(*next_c);
                    chars.next();
                }
                // arg.push(chars.peek().unwrap());
            }
            ' ' | '\t' | '\n' if !is_single_quotes && !is_double_quotes => {
                if !arg.is_empty() {
                    args.push(arg.to_owned());
                    arg = String::new();
                }
            }
            _ => arg.push(c),
        }
    }
    if !arg.is_empty() {
        args.push(arg);
    }
    args
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let full_command = parse_shell_args(&input); //input.split_whitespace().collect();
        let full_command: Vec<&str> = full_command.iter().map(|s| s.as_ref()).collect();
        match full_command.as_slice() {
            [] => continue,
            ["exit"] => break,
            ["echo", rest @ ..] => println!("{}", rest.join(" ")),
            ["pwd"] => println!("{}", builtin::pwd().display()),
            ["cd", dir] => {
                if let Err(e) = builtin::cd(*dir) {
                    eprintln!("cd: {}", e);
                }
            }
            ["cd"] => {
                if let Err(e) = builtin::cd("~") {
                    eprintln!("cd: {}", e);
                }
            }
            ["type", rest @ ("exit" | "echo" | "type" | "pwd")] => {
                println!("{} is a shell builtin", rest)
            }
            ["type", rest @ ..] => match get_type_of_command(rest[0]) {
                TypeCommand::Program(custom_exe) => println!("{} is {}", rest[0], custom_exe),
                _ => println!("{}: not found", rest[0]),
            },
            [commands @ ..] => match get_type_of_command(commands[0]) {
                TypeCommand::Program(_) => execute(commands[0], &commands[1..]),
                _ => println!("{}: command not found", commands[0]),
            },
        }
    }
}
