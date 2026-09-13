//! Output helpers: color handling, styles, and the renderers (tree, table, DOT, Mermaid).

pub mod dot;
pub mod mermaid;
pub mod table;
pub mod tree;

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use anstyle::{AnsiColor, Style};
use anyhow::{Context, Result};

/// `--color` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum ColorChoice {
    /// Color when writing to a terminal (respects `NO_COLOR` and `CLICOLOR_FORCE`).
    #[default]
    Auto,
    Always,
    Never,
}

impl ColorChoice {
    /// Apply globally; `anstream::stdout()` / `stderr()` then honor it.
    pub fn apply(self) {
        let choice = match self {
            ColorChoice::Auto => anstream::ColorChoice::Auto,
            ColorChoice::Always => anstream::ColorChoice::Always,
            ColorChoice::Never => anstream::ColorChoice::Never,
        };
        choice.write_global();
    }
}

/// The handful of styles `fl` uses. Rendered text embeds ANSI codes; `anstream` strips them when
/// the destination is not a terminal (or when writing to a file), so renderers never branch on
/// color themselves.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub key: Style,
    pub name: Style,
    pub dim: Style,
    pub heading: Style,
    pub error: Style,
    pub warning: Style,
    pub ok: Style,
    pub added: Style,
    pub removed: Style,
    pub highlight: Style,
    pub category: Style,
    pub data_use: Style,
    pub subject: Style,
    pub system: Style,
    pub dataset: Style,
}

impl Default for Theme {
    fn default() -> Self {
        let c = |color: AnsiColor| Style::new().fg_color(Some(color.into()));
        Theme {
            key: Style::new().bold(),
            name: Style::new(),
            dim: Style::new().dimmed(),
            heading: Style::new().bold().underline(),
            error: c(AnsiColor::Red).bold(),
            warning: c(AnsiColor::Yellow).bold(),
            ok: c(AnsiColor::Green).bold(),
            added: c(AnsiColor::Green),
            removed: c(AnsiColor::Red),
            highlight: c(AnsiColor::Yellow).bold(),
            category: c(AnsiColor::Cyan),
            data_use: c(AnsiColor::Magenta),
            subject: c(AnsiColor::Blue),
            system: c(AnsiColor::Blue).bold(),
            dataset: c(AnsiColor::Green).bold(),
        }
    }
}

impl Theme {
    pub fn paint(&self, style: Style, text: &str) -> String {
        format!("{style}{text}{style:#}")
    }

    pub fn for_kind(&self, kind: crate::taxonomy::Kind) -> Style {
        match kind {
            crate::taxonomy::Kind::Category => self.category,
            crate::taxonomy::Kind::Use => self.data_use,
            crate::taxonomy::Kind::Subject => self.subject,
        }
    }
}

/// Where command output goes: stdout (color-aware) or a file (ANSI always stripped).
pub enum Output {
    Stdout(anstream::AutoStream<io::Stdout>),
    File(anstream::StripStream<Box<dyn Write>>),
}

impl Output {
    pub fn open(path: Option<&Path>) -> Result<Output> {
        match path {
            Some(p) if p != Path::new("-") => {
                let f =
                    File::create(p).with_context(|| format!("cannot create {}", p.display()))?;
                Ok(Output::File(anstream::StripStream::new(
                    Box::new(BufWriter::new(f)) as Box<dyn Write>,
                )))
            }
            _ => Ok(Output::Stdout(anstream::stdout())),
        }
    }

    /// True when writing to a real file (used to suppress interactive niceties).
    pub fn is_file(&self) -> bool {
        matches!(self, Output::File(_))
    }
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Output::Stdout(s) => s.write(buf),
            Output::File(f) => f.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Output::Stdout(s) => s.flush(),
            Output::File(f) => f.flush(),
        }
    }
}

/// Strip ANSI escapes (for measuring widths).
pub fn plain(text: &str) -> String {
    String::from_utf8_lossy(&anstream::adapter::strip_bytes(text.as_bytes()).into_vec())
        .into_owned()
}

/// Display width of text after stripping ANSI escapes.
pub fn width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(plain(text).as_str())
}

/// Truncate `text` to `max` display columns, appending `…` when cut.
pub fn truncate(text: &str, max: usize) -> String {
    if width(text) <= max {
        return text.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for ch in text.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > max.saturating_sub(1) {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}
