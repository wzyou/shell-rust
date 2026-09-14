use anyhow::{Context, Result};

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Write, stderr, stdout},
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
    if is_builtin_cmd(command) {
        return TypeCommand::Builtin(command.to_string());
    }
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
    Builtin(String),
    Program(String),
    None,
}

const BUILTIN_COMMANDS: &[&str] = &[
    "cd", "exit", "echo", "type", "pwd", "complete", "jobs", "history", "declare",
];

pub fn is_builtin_cmd(cmd: &str) -> bool {
    BUILTIN_COMMANDS.contains(&cmd)
}

pub fn builtin_cmds() -> Vec<String> {
    BUILTIN_COMMANDS.iter().map(|c| c.to_string()).collect()
}

pub fn create_redirect(redirect: Option<Redirect>) -> Option<Box<dyn Write>> {
    if let Some(r) = redirect {
        match r {
            Redirect::Normal(f) => match File::create(&f) {
                Ok(file) => Some(Box::new(file)),
                Err(e) => {
                    eprintln!("无法创建文件 {}: {}", f, e);
                    None
                }
            },
            Redirect::Append(f) => match OpenOptions::new().create(true).append(true).open(&f) {
                Ok(file) => Some(Box::new(file)),
                Err(e) => {
                    eprintln!("无法创建文件 {}: {}", f, e);
                    None
                }
            },
        }
    } else {
        None
    }
}

pub fn execute_with_redirect(
    context: &mut context::Context,
    rl: &mut rustyline::Editor<crate::shellhelper::ShellHelper, rustyline::history::FileHistory>,
    args: &[&str],
    mut out: Option<Box<dyn Write>>,
    mut err: Option<Box<dyn Write>>,
) -> anyhow::Result<()> {
    // let mut io_out: Box<dyn Write> = match out {
    //     Some(redirect) => create_redirect(&redirect)?,
    //     None => Box::new(std::io::stdout()),
    // };
    // let mut io_err: Box<dyn Write> = match err {
    //     Some(redirect) => create_redirect(&redirect)?,
    //     None => Box::new(std::io::stderr()),
    // };
    let mut binding = stdout();
    let out = out.as_mut().map(|w| &mut **w).unwrap_or(&mut binding);
    let mut binding = stderr();
    let err = err.as_mut().map(|w| &mut **w).unwrap_or(&mut binding);

    match args {
        ["echo", rest @ ..] => {
            writeln!(out, "{}", rest.join(" "))?;
        }
        ["pwd"] => writeln!(out, "{}", pwd().display())?,
        ["cd", dir] => {
            if let Err(e) = cd(*dir) {
                writeln!(err, "cd: {}", e)?;
            }
        }
        ["cd"] => {
            if let Err(e) = cd("~") {
                writeln!(err, "cd: {}", e)?;
            }
        }
        ["complete", rest @ ..] => {
            if !rest.is_empty() {
                if rest[0] == "-p" && rest.len() >= 2 {
                    // writeln!(io_out, "complete: {}: no completion specification", rest[1])?;
                    let cmd = rest[1];
                    match context.regist_cmd_complete.get_key_value(cmd) {
                        Some((_, compelte)) => writeln!(out, "complete -C '{}' {}", compelte, cmd)?,
                        None => {
                            writeln!(out, "complete: {}: no completion specification", rest[1])?
                        }
                    }
                } else if rest[0] == "-C" && rest.len() >= 3 {
                    let (cmd, compelte) = (rest[2], rest[1]);
                    context
                        .regist_cmd_complete
                        .insert(cmd.to_string(), compelte.to_string());
                } else if rest.len() >= 2 && rest[0] == "-r" {
                    context.regist_cmd_complete.remove(rest[1]);
                }
            }
        }
        ["jobs", _rest @ ..] => {
            context.list_tasks();
        }
        ["history", "-r", file, ..] => {
            rl.load_history(file)?;
        }
        ["history", "-w", file, ..] => {
            let mut s: Vec<String> = rl.history().iter().map(|c| c.to_owned()).collect();
            s.push("".to_string());
            fs::write(file, s.join("\n"))?;
        }
        ["history", "-a", file, ..] => {
            // rl.append_history(file)?;
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(file)
                .and_then(|f| {
                    //
                    let history_count_in_file = context
                        .history_count_in_file
                        .entry(file.to_string())
                        .or_insert(0);

                    let history_count = history_count_in_file.clone();

                    *history_count_in_file = rl.history().iter().count();
                    for (i, item) in rl.history().iter().enumerate() {
                        if i >= history_count {
                            writeln!(&f, "{}", item)?;
                        }
                    }
                    Ok(())
                })
                .with_context(|| format!("无法打开文件 {}", file))?;
            // rl.clear_history()?;
        }
        ["history", rest @ ..] => {
            let history_items: Vec<_> = rl.history().iter().enumerate().collect();
            let history_items = if !rest.is_empty() {
                let n = rest[0].parse::<usize>().ok().unwrap();
                let len = history_items.len();
                history_items
                    .iter()
                    .filter(|(i, _)| i + n >= len)
                    .copied()
                    .collect()
            } else {
                history_items
            };

            for (i, item) in history_items {
                writeln!(out, "{:>5}  {}", i + 1, item)?;
            }
        }
        ["declare", rest @ ..] => match rest {
            ["-p", var, ..] => match context.vars.get_key_value(&var.to_string()) {
                Some((k, v)) => {
                    writeln!(out, "declare -x {}=\"{}\"", k, v)?;
                }
                None => {
                    writeln!(out, "declare: {}: not found", var)?;
                }
            },
            _ => {
                for (var, value) in &context.vars {
                    writeln!(out, "declare -x {}=\"{}\"", var, value)?;
                }
            }
        },
        ["type", command] if BUILTIN_COMMANDS.contains(&command) => {
            writeln!(out, "{} is a shell builtin", command)?;
        }
        ["type", rest @ ..] => match get_type_of_command(rest[0]) {
            TypeCommand::Program(custom_exe) => {
                writeln!(out, "{} is {}", rest[0], custom_exe)?;
            }
            _ => {
                writeln!(out, "{}: not found", rest[0])?;
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
