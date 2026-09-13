use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Helper, Result};

use crate::context;

pub struct ShellHelper {
    commands: Vec<String>,
    ext_executes: Vec<String>,
    filename_completer: FilenameCompleter,
    context: Rc<RefCell<context::Context>>,
}

impl ShellHelper {
    pub fn new(
        commands: Vec<String>,
        ext_cmds: Vec<String>,
        f_completer: FilenameCompleter,
        context: Rc<RefCell<context::Context>>,
    ) -> Self {
        Self {
            commands,
            ext_executes: ext_cmds,
            filename_completer: f_completer,
            context: context,
        }
    }
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

impl ShellHelper {
    fn complete_custem(&self, line: &str, pos: usize, start: usize) -> Option<(usize, Vec<Pair>)> {
        let args: Vec<&str> = line[..pos].split(" ").collect();
        if !args.is_empty() {
            let cmd = args[0];
            if self.commands.contains(&cmd.to_string())
                || self.ext_executes.contains(&cmd.to_string())
            {
                let ctx = &*self.context.borrow();
                if let Some((_, v)) = ctx.regist_cmd_complete.get_key_value(cmd) {
                    let (cmd, cur_arg, pre_arg) = (cmd, &line[start..pos], {
                        if args.len() >= 2 {
                            args[args.len() - 2]
                        } else {
                            ""
                        }
                    });
                    if let Ok(output) = Command::new(v)
                        .env("COMP_LINE", line)
                        .env("COMP_POINT", pos.to_string())
                        .arg(cmd)
                        .arg(cur_arg)
                        .arg(pre_arg)
                        .output()
                    {
                        let stdout_str = String::from_utf8_lossy(&output.stdout);
                        let matches = stdout_str
                            .lines()
                            .map(|line| Pair {
                                display: line.to_string(),
                                replacement: line.to_string() + " ",
                            })
                            .collect();
                        return Some((start, matches));
                    }
                }
            }
        }
        None
    }
}

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

        if let Some((size, matches)) = self.complete_custem(line, pos, start) {
            return Ok((size, matches));
        }

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
