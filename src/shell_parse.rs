pub fn parse_shell_args(intput: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut arg = String::new();
    let mut is_single_quotes = false;
    let mut is_double_quotes = false;

    let mut chars = intput.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' if !is_double_quotes => is_single_quotes = !is_single_quotes,
            '"' if !is_single_quotes => is_double_quotes = !is_double_quotes,
            '\\' if !is_single_quotes => {
                if let Some(next_c) = chars.peek() {
                    arg.push(*next_c);
                    chars.next();
                }
                // arg.push(chars.peek().unwrap());
            }
            '&' | ' ' | '\t' | '\n' if !is_single_quotes && !is_double_quotes => {
                if !arg.is_empty() {
                    args.push(arg);
                    arg = String::new();
                }
                if c == '&' {
                    args.push(c.to_string());
                }
            }
            '>' if !is_double_quotes && !is_single_quotes => {
                if !arg.is_empty() {
                    if arg.eq("1") {
                        arg = String::new();
                        if let Some('>') = chars.peek() {
                            args.push("1>>".to_string());
                            chars.next();
                        } else {
                            args.push("1>".to_string());
                        }
                    } else if arg.eq("2") {
                        arg = String::new();
                        if let Some('>') = chars.peek() {
                            args.push("2>>".to_string());
                            chars.next();
                        } else {
                            args.push("2>".to_string());
                        }
                    } else {
                        args.push(arg);
                        arg = String::new();
                        if let Some('>') = chars.peek() {
                            args.push("1>>".to_string());
                            chars.next();
                        } else {
                            args.push("1>".to_string());
                        }
                    }
                } else {
                    if let Some('>') = chars.peek() {
                        args.push("1>>".to_string());
                        chars.next();
                    } else {
                        args.push("1>".to_string());
                    }
                }
            }
            _ => arg.push(c),
        }
    }
    if !arg.is_empty() {
        args.push(arg);
    }
    args
}

pub enum Redirect {
    Normal(String),
    Append(String),
}

pub fn split_redirect(args: &[&str]) -> (Vec<String>, Option<Redirect>, Option<Redirect>) {
    let mut command_args = Vec::new();
    let mut redirect_file = None;
    let mut redirect_file_err = None;
    let mut iter = args.into_iter().copied().peekable();

    while let Some(arg) = iter.next() {
        match (arg, iter.peek().copied()) {
            ("1", Some(">")) => {
                panic!("error: 1>")
            }
            ("1>", Some(file)) => {
                redirect_file = Some(Redirect::Normal(file.to_owned()));
                iter.next();
            }
            ("2>", Some(file)) => {
                redirect_file_err = Some(Redirect::Normal(file.to_owned()));
                iter.next();
            }
            ("1>>", Some(file)) => {
                redirect_file = Some(Redirect::Append(file.to_owned()));
                iter.next();
            }
            ("2>>", Some(file)) => {
                redirect_file_err = Some(Redirect::Append(file.to_owned()));
                iter.next();
            }
            (arg, _) => {
                command_args.push(arg.to_owned());
            }
        }
    }

    (command_args, redirect_file, redirect_file_err)
}
