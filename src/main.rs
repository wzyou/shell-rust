use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Editor, Helper, Result};
use std::result::Result::Ok;

use crate::program::get_ext_executes;
mod shell_parse;

mod builtin;
mod program;

struct ShellHelper {
    commands: Vec<String>,
    ext_executes: Vec<String>,
    filename_completer: FilenameCompleter,
}

impl Hinter for ShellHelper {
    type Hint = String;
    // ...
}
impl Highlighter for ShellHelper {
    // ...
}
impl Helper for ShellHelper {}

impl Validator for ShellHelper {}
impl Completer for ShellHelper {
    type Candidate = Pair;
    fn complete(
        &self, // FIXME should be `&mut self`
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>)> {
        let matches;

        let start = line[..pos]
            .rfind(|c: char| c.is_whitespace())
            .map_or(0, |i| i + 1);

        let word = &line[start..pos];

        let mut cmds = self.commands.clone();
        cmds.append(&mut self.ext_executes.to_vec());

        if start == 0 {
            matches = cmds
                .iter()
                .filter(|&cmd| cmd.starts_with(word))
                .map(|cmd| Pair {
                    display: cmd.to_owned(),
                    replacement: cmd.to_string() + " ",
                })
                .collect();
        } else {
            return self
                .filename_completer
                .complete_path(line, pos)
                .map(|(size, ps)| {
                    (
                        size,
                        ps.iter()
                            .map(|p| Pair {
                                display: p.replacement.clone(),
                                replacement: match &p.replacement {
                                    r if r.ends_with("/") => r.to_string(),
                                    r => r.to_string() + " ",
                                },
                            })
                            .collect(),
                    )
                });
        }

        Ok((start, matches))
    }
}

fn main() -> anyhow::Result<()> {
    let mut rl = Editor::<ShellHelper, DefaultHistory>::new()?;

    let helper = ShellHelper {
        commands: vec![
            "pwd".to_string(),
            "exit".to_string(),
            "cd".to_string(),
            "type".to_string(),
            "echo".to_string(),
        ],
        ext_executes: get_ext_executes()?,
        filename_completer: FilenameCompleter::new(),
    };

    rl.set_helper(Some(helper));
    rl.set_completion_type(CompletionType::List);

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(&line)?;
                if let Ok(true) = line_deal(&line) {
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

fn line_deal(input: &str) -> anyhow::Result<bool> {
    let full_command = shell_parse::parse_shell_args(&input); //input.split_whitespace().collect();
    let full_command: Vec<&str> = full_command.iter().map(|s| s.as_ref()).collect();
    let (full_command, redirect_file, redirect_file_err) =
        shell_parse::split_redirect(&full_command);
    let args_ref: Vec<&str> = full_command.iter().map(|s| s.as_str()).collect();

    match args_ref.as_slice() {
        [] => {}
        ["exit"] => return Ok(true),
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
        [commands @ ..] => match builtin::get_type_of_command(commands[0]) {
            builtin::TypeCommand::Program(_) => {
                if let Err(e) = program::execute(
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
    Ok(false)
}
