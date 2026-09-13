use std::{cell::RefCell, rc::Rc};

use rustyline::{
    CompletionType, Editor, completion::FilenameCompleter, config::Configurer,
    error::ReadlineError, history::DefaultHistory,
};

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
        loop {
            let readline = self.rl.readline("$ ");

            match readline {
                Ok(line) => {
                    let line = line.trim();
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

    fn line_deal(&mut self, input: &str) -> anyhow::Result<bool> {
        let full_command = shell_parse::parse_shell_args(&input); //input.split_whitespace().collect();
        let full_command: Vec<&str> = full_command.iter().map(|s| s.as_ref()).collect();
        let (full_command, redirect_file, redirect_file_err) =
            shell_parse::split_redirect(&full_command);
        let args_ref: Vec<&str> = full_command.iter().map(|s| s.as_str()).collect();

        match args_ref.as_slice() {
            [] => {}
            ["exit"] => return Ok(true),
            [cmd, ..] if builtin::is_builtin_cmd(cmd) => {
                match builtin::execute_with_redirect(
                    &mut *self.context.to_owned().borrow_mut(),
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
                            redirect_file,
                            redirect_file_err,
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
