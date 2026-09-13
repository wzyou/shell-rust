use std::collections::HashMap;

pub struct Context {
    pub regist_cmd_complete: HashMap<String, String>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            regist_cmd_complete: HashMap::new(),
        }
    }
}
