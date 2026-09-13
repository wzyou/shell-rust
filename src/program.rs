use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::PermissionsExt,
    process::{Command, ExitStatus, Stdio},
};

use crate::builtin::get_path;
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

pub fn get_ext_executes() -> anyhow::Result<Vec<String>> {
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
    Ok(execs)
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
