use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::PermissionsExt,
    process::{Child, Command, ExitStatus, Stdio},
};

use crate::builtin::get_path;
use crate::shell_parse::Redirect;

fn execute_with_io<T: Into<Stdio>, U: Into<Stdio>>(
    custom_exe: &str,
    args: &[&str],
    stdout: Option<T>,
    stderr: Option<U>,
) -> anyhow::Result<ExitStatus> {
    let mut c = Command::new(custom_exe);
    if let Some(out) = stdout {
        c.stdout(out);
    }

    if let Some(out) = stderr {
        c.stderr(out);
    }

    match c.args(args).status() {
        Ok(exit_status) => Ok(exit_status),
        Err(e) => Err(e.into()),
    }
}

fn spawn_with_io<T: Into<Stdio>, U: Into<Stdio>>(
    custom_exe: &str,
    args: &[&str],
    stdout: Option<T>,
    stderr: Option<U>,
    pipe_input: Option<Stdio>,
) -> anyhow::Result<Child> {
    let mut c = Command::new(custom_exe);
    if let Some(out) = stdout {
        c.stdout(out);
    }

    if let Some(out) = stderr {
        c.stderr(out);
    }

    if let Some(input) = pipe_input {
        c.stdin(input);
    }

    match c.args(args).spawn() {
        Ok(child) => Ok(child),
        Err(e) => Err(e.into()),
    }
}

pub fn get_ext_executes() -> Vec<String> {
    let paths = get_path();
    let mut execs: Vec<String> = paths
        .iter()
        .filter_map(|path| fs::read_dir(path).ok())
        .flat_map(|entries| {
            entries.filter_map(|f| {
                let Ok(file) = f else {
                    return None;
                };
                if let (Some(file_name), Ok(metadata)) =
                    (file.file_name().to_str(), file.metadata())
                {
                    if metadata.permissions().mode() & 0o111 != 0 {
                        // println!("{} is {}/{}", file_name, path, file_name);
                        // break 'search;
                        // return TypeCommand::Program(format!("{}/{}", path, file_name));
                        return Some(file_name.to_string());
                    }
                }
                None
            })
        })
        .collect();
    // 1. 先排序（dedup 只能对相邻的重复元素生效）
    execs.sort();
    // 2. 再去重
    execs.dedup();
    execs
}

pub fn execute(
    custom_exe: &str,
    args: &[&str],
    redirect_stdout: Option<Redirect>,
    redirect_stderr: Option<Redirect>,
) -> anyhow::Result<ExitStatus> {
    let io_out = redirect_create(redirect_stdout);
    let io_err = redirect_create(redirect_stderr);

    execute_with_io(custom_exe, args, io_out, io_err)
}

pub fn spawn(
    custom_exe: &str,
    args: &[&str],
    pipe_input: Option<Stdio>,
    redirect_stdout: Option<Stdio>,
    redirect_stderr: Option<Stdio>,
) -> anyhow::Result<Child> {
    spawn_with_io(
        custom_exe,
        args,
        redirect_stdout,
        redirect_stderr,
        pipe_input,
    )
}

pub fn redirect_create(redirect: Option<Redirect>) -> Option<Stdio> {
    if let Some(r) = redirect {
        match r {
            Redirect::Normal(f) => match File::create(&f) {
                Ok(file) => Some(Stdio::from(file)),
                Err(e) => {
                    eprintln!("无法创建文件 {}: {}", &f, e);
                    None
                }
            },
            Redirect::Append(f) => match OpenOptions::new().create(true).append(true).open(&f) {
                Ok(file) => Some(Stdio::from(file)),
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
