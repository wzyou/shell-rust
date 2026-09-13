use std::{collections::HashMap, process::Child};

pub struct Task {
    id: u32,
    args: Vec<String>,
    child: Child,
    flush_flag: char,
}
pub struct Context {
    pub regist_cmd_complete: HashMap<String, String>,
    pub tasks: Vec<Task>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            regist_cmd_complete: HashMap::new(),
            tasks: Vec::new(),
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
                println!("[{}] {:<24}{}", t.id, "Done", t.args.join(" "));
                false
            }
            Ok(None) => true,
            Err(e) => {
                eprintln!("error checking tasks {} {}: {}", t.id, t.args.join(" "), e);
                false
            }
        });
    }

    pub fn list_tasks(&self) {
        self.tasks.iter().for_each(|t| {
            println!(
                "[{}]{} {:<24}{}",
                t.id,
                t.flush_flag,
                "Running",
                t.args.join(" ")
            );
        });
    }
}
