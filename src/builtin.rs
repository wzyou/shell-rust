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

fn handle_echo(
    context: &context::Context,
    args: &[&str],
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let rendered: Vec<String> = args
        .iter()
        .copied()
        .map(|arg| match arg.chars().collect::<Vec<_>>().as_slice() {
            ['$', var @ ..] => context
                .vars
                .get(&var.iter().collect::<String>())
                .cloned()
                .unwrap_or_default(),
            _ => arg.to_string(),
        })
        .collect();

    writeln!(out, "{}", rendered.join(" "))?;
    Ok(())
}

fn handle_complete(
    context: &mut context::Context,
    args: &[&str],
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    if args.is_empty() {
        return Ok(());
    }

    if args[0] == "-p" && args.len() >= 2 {
        let cmd = args[1];
        match context.regist_cmd_complete.get_key_value(cmd) {
            Some((_, complete)) => writeln!(out, "complete -C '{}' {}", complete, cmd)?,
            None => writeln!(out, "complete: {}: no completion specification", args[1])?,
        }
    } else if args[0] == "-C" && args.len() >= 3 {
        let (cmd, complete) = (args[2], args[1]);
        context
            .regist_cmd_complete
            .insert(cmd.to_string(), complete.to_string());
    } else if args.len() >= 2 && args[0] == "-r" {
        context.regist_cmd_complete.remove(args[1]);
    }

    Ok(())
}

fn handle_history(
    context: &mut context::Context,
    rl: &mut rustyline::Editor<crate::shellhelper::ShellHelper, rustyline::history::FileHistory>,
    args: &[&str],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> anyhow::Result<()> {
    match args {
        ["-r", file, ..] => {
            rl.load_history(file)?;
        }
        ["-w", file, ..] => {
            let mut s: Vec<String> = rl.history().iter().map(|c| c.to_owned()).collect();
            s.push(String::new());
            fs::write(file, s.join("\n"))?;
        }
        ["-a", file, ..] => {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(file)
                .and_then(|f| {
                    let history_count_in_file = context
                        .history_count_in_file
                        .entry(file.to_string())
                        .or_insert(0);
                    let history_count = *history_count_in_file;
                    *history_count_in_file = rl.history().iter().count();

                    for (i, item) in rl.history().iter().enumerate() {
                        if i >= history_count {
                            writeln!(&f, "{}", item)?;
                        }
                    }
                    Ok(())
                })
                .with_context(|| format!("无法打开文件 {}", file))?;
        }
        [rest @ ..] => {
            let history_items: Vec<_> = rl.history().iter().enumerate().collect();
            let history_items = if !rest.is_empty() {
                let Some(n) = rest[0].parse::<usize>().ok() else {
                    writeln!(err, "history: {}: invalid number", rest[0])?;
                    return Ok(());
                };
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
    }

    Ok(())
}

fn handle_declare(
    context: &mut context::Context,
    args: &[&str],
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    match args {
        ["-p", var, ..] => match context.vars.get_k_v(&var.to_string()) {
            Some((k, v)) => writeln!(out, "declare -- {}=\"{}\"", k, v)?,
            None => writeln!(out, "declare: {}: not found", var)?,
        },
        [var, ..] => {
            let parts: Vec<&str> = var.split('=').collect();
            if parts.len() == 2 {
                let (k, v) = (parts[0], parts[1]);
                if k.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    if let Some(first) = k.chars().next() {
                        if !first.is_ascii_digit() {
                            context.vars.set_var(k, v);
                            return Ok(());
                        }
                    }
                }
                writeln!(out, "declare: `{}={}\': not a valid identifier", k, v)?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn handle_type(args: &[&str], out: &mut dyn Write) -> anyhow::Result<()> {
    match args {
        [command] if BUILTIN_COMMANDS.contains(&command) => {
            writeln!(out, "{} is a shell builtin", command)?;
        }
        [command, ..] => match get_type_of_command(command) {
            TypeCommand::Program(custom_exe) => {
                writeln!(out, "{} is {}", command, custom_exe)?;
            }
            _ => writeln!(out, "{}: not found", command)?,
        },
        [] => {}
    }

    Ok(())
}

pub fn execute_with_redirect(
    context: &mut context::Context,
    rl: &mut rustyline::Editor<crate::shellhelper::ShellHelper, rustyline::history::FileHistory>,
    args: &[&str],
    mut out: Option<Box<dyn Write>>,
    mut err: Option<Box<dyn Write>>,
) -> anyhow::Result<()> {
    let mut binding = stdout();
    let out = out.as_mut().map(|w| &mut **w).unwrap_or(&mut binding);
    let mut binding = stderr();
    let err = err.as_mut().map(|w| &mut **w).unwrap_or(&mut binding);

    match args {
        ["echo", rest @ ..] => handle_echo(context, rest, out)?,
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
        ["complete", rest @ ..] => handle_complete(context, rest, out)?,
        ["jobs", _rest @ ..] => {
            context.list_tasks();
        }
        ["history", rest @ ..] => handle_history(context, rl, rest, out, err)?,
        ["declare", rest @ ..] => handle_declare(context, rest, out)?,
        ["type", rest @ ..] => handle_type(rest, out)?,
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
