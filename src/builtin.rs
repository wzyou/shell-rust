use anyhow::{Context, Result};

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

use crate::{context, shell_parse::Redirect};

pub fn get_path() -> Vec<String> {
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

pub enum TypeCommand {
    // Builtin(String),
    Program(String),
    None,
}

const BUILTIN_COMMANDS: &[&str] = &["cd", "exit", "echo", "type", "pwd", "complete"];

pub fn is_builtin_cmd(cmd: &str) -> bool {
    BUILTIN_COMMANDS.contains(&cmd)
}

pub fn builtin_cmds() -> Vec<String> {
    BUILTIN_COMMANDS.iter().map(|c| c.to_string()).collect()
}

pub fn execute_with_redirect(
    context: &mut context::Context,
    args: &[&str],
    out: Option<Redirect>,
    err: Option<Redirect>,
) -> anyhow::Result<()> {
    fn create_redirect(redirect: &Redirect) -> anyhow::Result<Box<dyn Write>> {
        match redirect {
            Redirect::Normal(f) => match File::create(&f) {
                Ok(file) => Ok(Box::new(file)),
                Err(e) => {
                    eprintln!("无法创建文件 {}: {}", f, e);
                    return Err(e.into()); // 遇到错误直接返回，不执行后续命令
                }
            },
            Redirect::Append(f) => {
                match OpenOptions::new().create(true).append(true).open(&f) {
                    Ok(file) => Ok(Box::new(file)),
                    Err(e) => {
                        eprintln!("无法创建文件 {}: {}", f, e);
                        return Err(e.into()); // 遇到错误直接返回，不执行后续命令
                    }
                }
            }
        }
    }
    let mut io_out: Box<dyn Write> = match out {
        Some(redirect) => create_redirect(&redirect)?,
        None => Box::new(std::io::stdout()),
    };
    let mut io_err: Box<dyn Write> = match err {
        Some(redirect) => create_redirect(&redirect)?,
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
        ["complete", rest @ ..] => {
            if !rest.is_empty() {
                if rest[0] == "-p" && rest.len() >= 2 {
                    // writeln!(io_out, "complete: {}: no completion specification", rest[1])?;
                    let cmd = rest[1];
                    match context.regist_cmd_complete.get_key_value(cmd) {
                        Some((_, compelte)) => {
                            writeln!(io_out, "complete -C '{}' {}", compelte, cmd)?
                        }
                        None => {
                            writeln!(io_out, "complete: {}: no completion specification", rest[1])?
                        }
                    }
                } else if rest[0] == "-C" && rest.len() >= 3 {
                    let (cmd, compelte) = (rest[2], rest[1]);
                    context
                        .regist_cmd_complete
                        .insert(cmd.to_string(), compelte.to_string());
                }
            }
        }
        ["type", command] if BUILTIN_COMMANDS.contains(&command) => {
            writeln!(io_out, "{} is a shell builtin", command)?;
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
