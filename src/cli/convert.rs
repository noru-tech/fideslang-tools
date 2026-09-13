//! `fl convert` — YAML ⇄ JSON ⇄ CSV for taxonomy files and manifests.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::Args as ClapArgs;

use super::{Ctx, FilterArgs};
use crate::Exit;
use crate::format::{self, Format};
use crate::manifest::Manifest;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Input file (`-` for stdin).
    pub input: PathBuf,
    /// Input format (default: from the extension; YAML for stdin).
    #[arg(long, value_enum)]
    pub from: Option<Format>,
    /// Output format (default: from the `-o` extension, else JSON for YAML input and YAML otherwise).
    #[arg(long, value_enum)]
    pub to: Option<Format>,
    #[command(flatten)]
    pub filter: FilterArgs,
}

pub fn run(ctx: &mut Ctx, a: Args) -> Result<Exit> {
    let from = a.from.unwrap_or_else(|| {
        if a.input == Path::new("-") {
            Format::Yaml
        } else {
            Format::from_path(&a.input)
        }
    });
    let to = match (a.to, ctx.global.output.as_deref()) {
        (Some(t), _) => t,
        (None, Some(o)) if o != Path::new("-") && o.extension().is_some() => Format::from_path(o),
        (None, _) => {
            if from == Format::Yaml {
                Format::Json
            } else {
                Format::Yaml
            }
        }
    };
    let value = format::read_value(&a.input, Some(from))?;
    let filter = a.filter.build()?;
    let value = if filter.is_noop() {
        value
    } else {
        format::expect_manifest_shape(&value, &a.input.display().to_string())?;
        let mut m = Manifest::new();
        m.add_document(value, Some(&a.input));
        filter.apply(&m).to_value()
    };
    if to == Format::Csv {
        format::expect_manifest_shape(&value, &a.input.display().to_string())?;
    }
    if value.is_null() {
        bail!("{}: empty document", a.input.display());
    }
    format::write_value(&mut ctx.out, &value, to)?;
    Ok(Exit::Ok)
}
