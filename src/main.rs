mod builtin;
mod context;
mod program;
mod shell;
mod shell_parse;
mod shellhelper;

fn main() -> anyhow::Result<()> {
    let mut shell = shell::Shell::new();
    shell.run()?;
    Ok(())
}
