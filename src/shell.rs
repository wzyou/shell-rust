use rustyline::{
    CompletionType, Editor, completion::FilenameCompleter, config::Configurer,
    error::ReadlineError, history::DefaultHistory,
};
use std::env;
use std::io::{self};
use std::{cell::RefCell, process::Stdio, rc::Rc};

use crate::{builtin, context, program, shell_parse, shellhelper};

pub struct Shell {
    rl: Editor<shellhelper::ShellHelper, rustyline::history::FileHistory>,
    context: Rc<RefCell<context::Context>>,
}

impl Shell {
    pub fn new() -> Self {
        let mut rl: Editor<shellhelper::ShellHelper, rustyline::history::FileHistory> =
            Editor::<shellhelper::ShellHelper, DefaultHistory>::new().unwrap();

        let context = Rc::new(RefCell::new(context::Context::new()));

        let helper = shellhelper::ShellHelper::new(
            builtin::builtin_cmds(),
            program::get_ext_executes(),
            FilenameCompleter::new(),
            context.clone(),
        );

        rl.set_helper(Some(helper));
        rl.set_completion_type(CompletionType::List);
        rl.set_bell_style(rustyline::config::BellStyle::Audible);

        Self {
            rl,
            context: context.clone().into(),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        env::var("HISTFILE").ok().map(|histfile| {
            self.rl.load_history(&histfile).unwrap_or_else(|_| {
                eprintln!("无法加载历史文件: {}", histfile);
            });
        });
        loop {
            self.context.borrow_mut().update_tasks();

            let readline = self.rl.readline("$ ");

            match readline {
                Ok(line) => {
                    let line = line.trim();
                    if line.starts_with("#") {
                        continue;
                    }
                    self.rl.add_history_entry(line)?;
                    if let Ok(true) = self.line_deal(line) {
                        break;
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    // break;
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

        Ok(())
    }

    fn pipe_deal(&mut self, pipe_commands: &[&str]) -> anyhow::Result<bool> {
        let mut prev_pipe: Option<Stdio> = None;
        let mut childen = Vec::new();

        if let Some(cmds) = shell_parse::split_pipe(pipe_commands) {
            for (i, args) in cmds.iter().enumerate() {
                let is_last = i == cmds.len() - 1;
                let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let (full_command, redirect_file, redirect_file_err) =
                    shell_parse::split_redirect(&args_str);
                let args_ref: Vec<&str> = full_command.iter().map(|s| s.as_str()).collect();

                match args_ref.as_slice() {
                    [commands @ ..] => match builtin::get_type_of_command(commands[0]) {
                        builtin::TypeCommand::Program(_) => {
                            let (stdout, stderr) = if is_last {
                                (
                                    program::redirect_create(redirect_file)
                                        .or(Some(Stdio::inherit())),
                                    program::redirect_create(redirect_file_err),
                                )
                            } else {
                                (Some(Stdio::piped()), None)
                            };
                            match program::spawn(
                                commands[0],
                                &commands[1..commands.len()],
                                prev_pipe.take(),
                                stdout,
                                stderr,
                            ) {
                                Ok(mut child) => {
                                    if !is_last {
                                        prev_pipe = child.stdout.take().map(|p| Stdio::from(p));
                                    } else {
                                        prev_pipe = None;
                                        child.stdout = None;
                                    }
                                    childen.push(child);
                                }
                                Err(e) => {
                                    eprintln!("ERR: {e}");
                                }
                            }
                        }
                        builtin::TypeCommand::Builtin(_cmd) => {
                            let (stdout, stderr) = if is_last {
                                (
                                    builtin::create_redirect(redirect_file),
                                    builtin::create_redirect(redirect_file_err),
                                )
                            } else {
                                // let pipe = Stdio::piped();
                                // writeln!(pipe, "pipe").unwrap();
                                // File::from(pipe.as_raw_fd());
                                let (stdout_reader, stdout_writer) =
                                    io::pipe().expect("create pipe failure");
                                prev_pipe = Some(Stdio::from(stdout_reader));
                                let stdout: Box<dyn io::Write> = Box::new(stdout_writer);
                                (Some(stdout), None)
                            };
                            match builtin::execute_with_redirect(
                                &mut *self.context.borrow_mut(),
                                &mut self.rl,
                                commands,
                                stdout,
                                stderr,
                            ) {
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("ERR: {e}");
                                }
                            }
                        }
                        _ => println!(
                            "{}: builtin command could not be used for pipe",
                            commands[0]
                        ),
                    },
                }
            }

            let _rs: Vec<bool> = childen.iter_mut().map(|c| c.wait().is_ok()).collect();

            Ok(false)
        } else {
            Ok(false)
        }
    }

    fn is_pepe_command(&self, command: &[&str]) -> bool {
        command.contains(&"|")
    }

    fn line_deal(&mut self, input: &str) -> anyhow::Result<bool> {
        let full_command = shell_parse::parse_shell_args(&input);
        let full_command: Vec<&str> = full_command.iter().map(|s| s.as_ref()).collect();
        if self.is_pepe_command(&full_command) {
            return self.pipe_deal(&full_command);
        }
        let (full_command, redirect_file, redirect_file_err) =
            shell_parse::split_redirect(&full_command);
        let args_ref: Vec<&str> = full_command.iter().map(|s| s.as_str()).collect();

        match args_ref.as_slice() {
            [] => {}
            ["exit"] => return Ok(true),
            [cmd, ..] if builtin::is_builtin_cmd(cmd) => {
                match builtin::execute_with_redirect(
                    &mut *self.context.to_owned().borrow_mut(),
                    &mut self.rl,
                    args_ref.as_slice(),
                    builtin::create_redirect(redirect_file),
                    builtin::create_redirect(redirect_file_err),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        eprint!("{e}");
                    }
                };
            }
            [commands @ ..] => match builtin::get_type_of_command(commands[0]) {
                builtin::TypeCommand::Program(_) => {
                    if commands[commands.len() - 1] != "&" {
                        if let Err(e) = program::execute(
                            commands[0],
                            &commands[1..],
                            redirect_file,
                            redirect_file_err,
                        ) {
                            eprintln!("ERR: {e}");
                        }
                    } else {
                        match program::spawn(
                            commands[0],
                            &commands[1..commands.len() - 1],
                            None,
                            program::redirect_create(redirect_file),
                            program::redirect_create(redirect_file_err),
                        ) {
                            Ok(child) => {
                                let pid = child.id();
                                let id = self.context.borrow_mut().add_task(
                                    child,
                                    commands.iter().map(|arg| arg.to_string()).collect(),
                                );
                                println!("[{}] {}", id, pid);
                            }
                            Err(e) => {
                                eprintln!("ERR: {e}");
                            }
                        }
                    }
                }
                _ => println!("{}: command not found", commands[0]),
            },
        }
        Ok(false)
    }
}
