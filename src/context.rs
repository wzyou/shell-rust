use std::{collections::HashMap, process::Child};

pub struct Task {
    id: u32,
    args: Vec<String>,
    child: Child,
    flush_flag: char,
}

pub struct Variables {
    pub vars: HashMap<String, String>,
}

#[derive(Debug)]
enum ExpansionType {
    Var(String),
    Text(String),
}

impl Variables {
    fn new() -> Self {
        Variables {
            vars: HashMap::new(),
        }
    }

    pub fn set_var(&mut self, k: &str, v: &str) {
        if v.chars().collect::<Vec<_>>()[0].eq(&'$') {
            let v_k = &v[1..];
            let v = &self.vars.get(v_k).unwrap_or(&"".to_string()).to_owned();
            self.set(k, v);
        } else {
            self.vars.insert(k.to_string(), v.to_string());
        }
    }

    pub fn set(&mut self, k: &str, v: &str) {
        self.vars.insert(k.to_string(), v.to_string());
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.vars.get(k)
    }

    fn parameter_expansion_parse(&self, inout: &str) -> Vec<ExpansionType> {
        // eprintln!("input: {:#?}", inout);
        let mut pee = inout.chars().peekable();

        let mut exps = Vec::new();
        let mut exp = String::new();

        let mut is_in_big_quotes = false;
        let mut is_meet_doler = false;

        while let Some(c) = pee.next() {
            match c {
                '$' => {
                    if exp.is_empty() {
                        is_meet_doler = true;
                        continue;
                    }

                    if is_meet_doler {
                        exps.push(ExpansionType::Var(exp));
                    } else {
                        exps.push(ExpansionType::Text(exp));
                    }
                    is_meet_doler = true;
                    exp = String::new();
                }
                '{' => {
                    if is_meet_doler {
                        if !exp.is_empty() {
                            exps.push(ExpansionType::Text(exp));
                            exp = String::new();
                        }
                        is_in_big_quotes = true;
                    } else {
                        exp.push(c);
                    }
                }
                '}' => {
                    if is_meet_doler {
                        if is_in_big_quotes {
                            exps.push(ExpansionType::Var(exp));
                            exp = String::new();
                            is_in_big_quotes = false;
                        } else {
                            exp.push(c);
                        }
                    }
                    is_meet_doler = false;
                }
                _ => {
                    exp.push(c);
                }
            }
        }
        if !exp.is_empty() {
            if is_meet_doler {
                exps.push(ExpansionType::Var(exp));
            } else {
                exps.push(ExpansionType::Text(exp));
            }
        }
        // eprintln!("exps: {:#?}", exps);
        exps
    }

    pub fn try_get(&self, k: &str) -> String {
        let ks: Vec<String> = self
            .parameter_expansion_parse(k)
            .iter()
            .map(|exp| match exp {
                ExpansionType::Var(var) => self.get(var).unwrap_or(&"".to_string()).to_string(),
                ExpansionType::Text(text) => text.to_owned(),
            })
            .collect();
        // eprintln!("ks: {:#?}", ks);
        ks.iter().flat_map(|s| s.chars()).collect()
    }

    pub fn get_k_v(&self, k: &str) -> Option<(&String, &String)> {
        self.vars.get_key_value(k)
    }
}

pub struct Context {
    pub regist_cmd_complete: HashMap<String, String>,
    pub tasks: Vec<Task>,
    pub history_count_in_file: HashMap<String, usize>,
    pub vars: Variables,
}

impl Context {
    pub fn new() -> Self {
        Self {
            regist_cmd_complete: HashMap::new(),
            tasks: Vec::new(),
            history_count_in_file: HashMap::new(),
            vars: Variables::new(),
        }
    }

    pub fn add_task(&mut self, child: Child, args: Vec<String>) -> u32 {
        let id = self.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;

        for t in &mut self.tasks {
            // t.flush_flag = '-';
            if t.flush_flag.eq(&'+') {
                t.flush_flag = '-';
            } else {
                t.flush_flag = ' ';
            }
        }

        self.tasks.push(Task {
            id,
            args,
            child,
            flush_flag: '+',
        });
        id
    }

    pub fn update_tasks(&mut self) {
        self.tasks.retain_mut(|t| match t.child.try_wait() {
            Ok(Some(_)) => {
                let args = &t.args[..t.args.len() - 1];
                println!(
                    "[{}]{} {:<24}{}",
                    t.id,
                    t.flush_flag,
                    "Done",
                    args.join(" ")
                );
                false
            }
            Ok(None) => true,
            Err(e) => {
                eprintln!("error checking tasks {} {}: {}", t.id, t.args.join(" "), e);
                false
            }
        });
        let count = self.tasks.len();
        for (i, t) in &mut self.tasks.iter_mut().enumerate() {
            // t.flush_flag = '-';
            if i == count - 1 {
                t.flush_flag = '+';
            } else if i == count - 2 {
                t.flush_flag = '-';
            }
        }
    }

    pub fn list_tasks(&mut self) {
        self.tasks.retain_mut(|t| match t.child.try_wait() {
            Ok(Some(_)) => {
                let args = &t.args[..t.args.len() - 1];
                println!(
                    "[{}]{} {:<24}{}",
                    t.id,
                    t.flush_flag,
                    "Done",
                    args.join(" ")
                );
                false
            }
            Ok(None) => {
                println!(
                    "[{}]{} {:<24}{}",
                    t.id,
                    t.flush_flag,
                    "Running",
                    t.args.join(" ")
                );
                true
            }
            Err(e) => {
                eprintln!("error checking tasks {} {}: {}", t.id, t.args.join(" "), e);
                false
            }
        });
        let count = self.tasks.len();
        for (i, t) in &mut self.tasks.iter_mut().enumerate() {
            // t.flush_flag = '-';
            if i == count - 1 {
                t.flush_flag = '+';
            } else if i == count - 2 {
                t.flush_flag = '-';
            }
        }
    }
}
