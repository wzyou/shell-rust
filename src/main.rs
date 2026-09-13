// use anyhow::Ok as AOK;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::result::Result::Ok;
use std::{
    fs::File,
    process::{Command, ExitStatus, Stdio},
};

// use anyhow::Ok;
// use anyhow::Result;

fn execute_with_io<T: Into<Stdio>, U: Into<Stdio>>(
    custom_exe: &str,
    args: &[&str],
    stdout: T,
    stderr: U,
) -> anyhow::Result<ExitStatus> {
    match Command::new(custom_exe)
        .stdout(stdout)
        .stderr(stderr)
        .args(args)
        .status()
    {
        Ok(exit_status) => Ok(exit_status),
        Err(e) => Err(e.into()),
    }
}

fn execute(
    custom_exe: &str,
    args: &[&str],
    redirect_stdout: Option<String>,
    redirect_stderr: Option<String>,
) -> anyhow::Result<ExitStatus> {
    let io_out = match redirect_stdout {
        Some(f) => {
            match File::create(&f) {
                Ok(file) => Stdio::from(file),
                Err(e) => {
                    eprintln!("无法创建文件 {}: {}", f, e);
                    return Err(e.into()); // 遇到错误直接返回，不执行后续命令
                }
            }
        }
        None => Stdio::inherit(),
    };
    let io_err = match redirect_stderr {
        Some(f) => {
            match File::create(&f) {
                Ok(file) => Stdio::from(file),
                Err(e) => {
                    eprintln!("无法创建文件 {}: {}", f, e);
                    return Err(e.into()); // 遇到错误直接返回，不执行后续命令
                }
            }
        }
        None => Stdio::inherit(),
    };

    execute_with_io(custom_exe, args, io_out, io_err)
}

mod builtin {
    use anyhow::{Context, Result};

    use std::{
        env,
        fs::{self, File},
        io::Write,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
    };

    fn get_path() -> Vec<String> {
        env::var("PATH")
            .unwrap_or("".to_string())
            .split(':')
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
    }

    pub fn get_type_of_command(command: &str) -> TypeCommand {
        let paths = get_path();
        for path in &paths {
            let Ok(entrues) = fs::read_dir(path) else {
                continue;
            };
            for _dir in entrues {
                let Ok(dir) = _dir else {
                    continue;
                };
                if let (Some(file_name), Ok(metadata)) = (dir.file_name().to_str(), dir.metadata())
                {
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

    pub enum TypeCommand {
        // Builtin(String),
        Program(String),
        None,
    }

    pub fn execute_with_redirect(
        args: &[&str],
        out: Option<String>,
        err: Option<String>,
    ) -> anyhow::Result<()> {
        let mut io_out: Box<dyn Write> = match out {
            Some(f) => {
                match File::create(&f) {
                    Ok(file) => Box::new(file),
                    Err(e) => {
                        eprintln!("无法创建文件 {}: {}", f, e);
                        return Err(e.into()); // 遇到错误直接返回，不执行后续命令
                    }
                }
            }
            None => Box::new(std::io::stdout()),
        };
        let mut io_err: Box<dyn Write> = match err {
            Some(f) => {
                match File::create(&f) {
                    Ok(file) => Box::new(file),
                    Err(e) => {
                        eprintln!("无法创建文件 {}: {}", f, e);
                        return Err(e.into()); // 遇到错误直接返回，不执行后续命令
                    }
                }
            }
            None => Box::new(std::io::stderr()),
        };
        match args {
            ["echo", rest @ ..] => {
                writeln!(io_out, "{}", rest.join(" "))?;
            }
            ["pwd"] => println!("{}", pwd().display()),
            ["cd", dir] => {
                if let Err(e) = cd(*dir) {
                    writeln!(io_err, "cd: {}", e)?;
                }
            }
            ["cd"] => {
                if let Err(e) = cd("~") {
                    writeln!(io_err, "cd: {}", e)?;
                }
            }
            ["type", rest @ ("exit" | "echo" | "type" | "pwd")] => {
                writeln!(io_out, "{} is a shell builtin", rest)?;
            }
            ["type", rest @ ..] => match get_type_of_command(rest[0]) {
                TypeCommand::Program(custom_exe) => {
                    writeln!(io_out, "{} is {}", rest[0], custom_exe)?;
                }
                _ => {
                    writeln!(io_out, "{}: not found", rest[0])?;
                }
            },
            _ => {}
        }
        Ok(())
    }

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
                    args.push(arg);
                    arg = String::new();
                }
            }
            '>' if !is_double_quotes && !is_single_quotes => {
                if !arg.is_empty() {
                    if arg.eq("1") {
                        arg = String::new();
                        args.push("1>".to_string());
                    } else if arg.eq("2") {
                        arg = String::new();
                        args.push("2>".to_string());
                    } else {
                        args.push(arg);
                        arg = String::new();
                        args.push("1>".to_string());
                    }
                } else {
                    args.push("1>".to_string());
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

fn split_redirect(args: &[&str]) -> (Vec<String>, Option<String>, Option<String>) {
    let mut command_args = Vec::new();
    let mut redirect_file = None;
    let mut redirect_file_err = None;
    let mut iter = args.into_iter().copied().peekable();

    while let Some(arg) = iter.next() {
        match (arg, iter.peek().copied()) {
            ("1", Some(">")) => {
                panic!("error: 1>")
            }
            ("1>", Some(file)) => {
                redirect_file = Some(file.to_owned());
                iter.next();
            }
            ("2>", Some(file)) => {
                redirect_file_err = Some(file.to_owned());
                iter.next();
            }
            (arg, _) => {
                command_args.push(arg.to_owned());
            }
        }
    }

    (command_args, redirect_file, redirect_file_err)
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let full_command = parse_shell_args(&input); //input.split_whitespace().collect();
        let full_command: Vec<&str> = full_command.iter().map(|s| s.as_ref()).collect();
        let (full_command, redirect_file, redirect_file_err) = split_redirect(&full_command);
        let args_ref: Vec<&str> = full_command.iter().map(|s| s.as_str()).collect();
        // let redirect_stdio = redirect_file
        //     .map(|f| Stdio::from(File::create(f).unwrap()))
        //     .unwrap_or_else(|| Stdio::inherit());
        match args_ref.as_slice() {
            [] => continue,
            ["exit"] => break,
            [cmd, ..] if matches!(*cmd, "echo" | "pwd" | "cd" | "type") => {
                match builtin::execute_with_redirect(
                    args_ref.as_slice(),
                    redirect_file,
                    redirect_file_err,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        eprint!("{e}");
                    }
                };
            }
            // ["echo", rest @ ..] => println!("{}", rest.join(" ")),
            // ["pwd"] => println!("{}", builtin::pwd().display()),
            // ["cd", dir] => {
            //     if let Err(e) = builtin::cd(*dir) {
            //         eprintln!("cd: {}", e);
            //     }
            // }
            // ["cd"] => {
            //     if let Err(e) = builtin::cd("~") {
            //         eprintln!("cd: {}", e);
            //     }
            // }
            // ["type", rest @ ("exit" | "echo" | "type" | "pwd")] => {
            //     println!("{} is a shell builtin", rest)
            // }
            // ["type", rest @ ..] => match builtin::get_type_of_command(rest[0]) {
            //     builtin::TypeCommand::Program(custom_exe) => {
            //         println!("{} is {}", rest[0], custom_exe)
            //     }
            //     _ => println!("{}: not found", rest[0]),
            // },
            [commands @ ..] => match builtin::get_type_of_command(commands[0]) {
                builtin::TypeCommand::Program(_) => {
                    if let Err(e) = execute(
                        commands[0],
                        &commands[1..],
                        redirect_file,
                        redirect_file_err,
                    ) {
                        eprintln!("ERR: {e}");
                    }
                }
                _ => println!("{}: command not found", commands[0]),
            },
        }
    }
}
