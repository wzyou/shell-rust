// use anyhow::Ok as AOK;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::result::Result::Ok;
mod shell_parse;

mod builtin;
mod program;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let full_command = shell_parse::parse_shell_args(&input); //input.split_whitespace().collect();
        let full_command: Vec<&str> = full_command.iter().map(|s| s.as_ref()).collect();
        let (full_command, redirect_file, redirect_file_err) =
            shell_parse::split_redirect(&full_command);
        let args_ref: Vec<&str> = full_command.iter().map(|s| s.as_str()).collect();

        match args_ref.as_slice() {
            [] => continue,
            ["exit"] => break,
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
    }
}
