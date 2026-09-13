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

        let cmd: Vec<&str> = line[..pos].split(" ").collect();
        if !cmd.is_empty() {
            let cmd = cmd[0];
            if cmds.contains(&cmd.to_string()) {
                let ctx = &*self.context.borrow();
                if let Some((_, v)) = ctx.regist_cmd_complete.get_key_value(cmd) {
                    if let Ok(output) = Command::new(v).output() {
                        let stdout_str = String::from_utf8_lossy(&output.stdout);
                        return Ok((
                            start,
                            stdout_str
                                .lines()
                                .filter(|l| l.starts_with(&word))
                                .map(|line| Pair {
                                    display: line.to_string(),
                                    replacement: line.to_string() + " ",
                                })
                                .collect(),
                        ));
                    }
                }
            }
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
