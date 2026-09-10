#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    let builtin_commands = [
        "exit",
        "echo",
        "type",
    ];
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        let command = command.trim().to_string();
        if command == "exit" {
            break;
        }
        if command.starts_with("echo ") {
            println!("{}", &command[5..]);
            continue;
        }
        if command.starts_with("type ") {
            let o_command = command[5..].to_string();
            if builtin_commands.iter().any(|&c| *c == *o_command ) {
                println!("{} is a shell builtin", o_command);
            } else {
                println!("{}: not found", o_command);
            }
            continue;
        }
        println!("{}: command not found", command);
    }
}
