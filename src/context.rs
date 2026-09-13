use std::{collections::HashMap, process::Child};

pub struct Task {
    id: u32,
    args: Vec<String>,
    child: Child,
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
        // let pid = child.id();
        self.tasks.push(Task { id, args, child });
        id
    }

    pub fn update_tasks(&mut self) {
        self.tasks.retain_mut(|t| match t.child.try_wait() {
            Ok(Some(_)) => {
                println!("[{}] done {}", t.id, t.args.join(" "));
                false
            }
            Ok(None) => true,
            Err(e) => {
                eprintln!("error checking tasks {} {}: {}", t.id, t.args.join(" "), e);
                false
            }
        });
    }
}
