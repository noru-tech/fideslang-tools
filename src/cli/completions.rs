//! `fl completions` and `fl manpage`

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args as ClapArgs, CommandFactory};
use clap_complete::Shell;

use super::{Cli, Ctx};
use crate::Exit;

#[derive(Debug, ClapArgs)]
pub struct Args {
    #[arg(value_enum)]
    pub shell: Shell,
}

pub fn run(ctx: &mut Ctx, a: Args) -> Result<Exit> {
    let mut cmd = Cli::command();
    let mut buf = Vec::new();
    clap_complete::generate(a.shell, &mut cmd, "fl", &mut buf);
    ctx.out.write_all(&buf)?;
    Ok(Exit::Ok)
}

#[derive(Debug, ClapArgs)]
pub struct ManArgs {
    /// Write one page per subcommand into DIR (default: the main page to stdout).
    #[arg(short = 'd', long, value_name = "DIR")]
    pub out_dir: Option<PathBuf>,
}

pub fn run_man(ctx: &mut Ctx, a: ManArgs) -> Result<Exit> {
    let cmd = Cli::command();
    match a.out_dir {
        None => {
            let mut buf = Vec::new();
            clap_mangen::Man::new(cmd).render(&mut buf)?;
            ctx.out.write_all(&buf)?;
        }
        Some(dir) => {
            fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
            clap_mangen::generate_to(cmd, &dir)?;
            let mut pages: Vec<String> = fs::read_dir(&dir)?
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.starts_with("fl") && n.ends_with(".1"))
                .collect();
            pages.sort();
            ctx.note(format!(
                "wrote {} man pages to {}: {}",
                pages.len(),
                dir.display(),
                pages.join(", ")
            ));
        }
    }
    Ok(Exit::Ok)
}
