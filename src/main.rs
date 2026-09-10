#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        
        let full_command:Vec<&str> = command.split_whitespace().collect();

        match full_command.as_slice() {
            [] => continue,
            ["exit"] => break,
            ["echo", rest @ ..] => println!("{}", rest.join(" ")),
            ["type", rest @ ("exit" | "echo" | "type")] => println!("{} is a shell builtin", rest),
            ["type", rest @ ..] =>  println!("{}: not found", rest[0]),
            _ => println!("{}: command not found", full_command[0]),
        }
    }
}
