use std::{
    fs::{File, OpenOptions},
    process::{Command, ExitStatus, Stdio},
};

use crate::shell_parse::Redirect;

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

pub fn execute(
    custom_exe: &str,
    args: &[&str],
    redirect_stdout: Option<Redirect>,
    redirect_stderr: Option<Redirect>,
) -> anyhow::Result<ExitStatus> {
    fn redirect_create(redirect: &Redirect) -> anyhow::Result<Stdio> {
        match redirect {
            Redirect::Normal(f) => match File::create(f) {
                Ok(file) => Ok(Stdio::from(file)),
                Err(e) => {
                    eprintln!("无法创建文件 {}: {}", f, e);
                    return Err(e.into()); // 遇到错误直接返回，不执行后续命令
                }
            },
            Redirect::Append(f) => match OpenOptions::new().create(true).append(true).open(&f) {
                Ok(file) => Ok(Stdio::from(file)),
                Err(e) => {
                    eprintln!("无法创建文件 {}: {}", f, e);
                    return Err(e.into()); // 遇到错误直接返回，不执行后续命令
                }
            },
        }
    }
    let io_out = match redirect_stdout {
        Some(redirect) => redirect_create(&redirect)?,
        None => Stdio::inherit(),
    };
    let io_err = match redirect_stderr {
        Some(redirect) => redirect_create(&redirect)?,
        None => Stdio::inherit(),
    };

    execute_with_io(custom_exe, args, io_out, io_err)
}
