//! E001 unknown taxonomy key · W001 deprecated key · W004 `data_purposes`.

use super::diag;
use crate::manifest::Manifest;
use crate::manifest::keys::taxonomy_uses;
use crate::taxonomy::Taxonomy;
use crate::validate::{Diagnostic, Severity, suggest};

pub fn run(manifest: &Manifest, tax: &Taxonomy, out: &mut Vec<Diagnostic>) {
    for r in manifest.iter() {
        for u in taxonomy_uses(r) {
            if u.field == "data_purposes" {
                out.push(diag(
                    r,
                    "W004",
                    Severity::Warning,
                    u.path.clone(),
                    "`data_purposes` is deprecated; use `data_uses`",
                ));
            }
            match tax.get(u.kind, &u.key) {
                None => {
                    let mut d = diag(
                        r,
                        "E001",
                        Severity::Error,
                        u.path.clone(),
                        format!("unknown {} `{}`", u.kind.human(), u.key),
                    );
                    d.suggestion = suggest::format(&suggest::suggest(&u.key, tax.table(u.kind), 3));
                    out.push(d);
                }
                Some(rec) if rec.is_deprecated() => {
                    let mut d = diag(
                        r,
                        "W001",
                        Severity::Warning,
                        u.path.clone(),
                        format!(
                            "{} `{}` is deprecated since {}",
                            u.kind.human(),
                            u.key,
                            rec.version_deprecated.as_deref().unwrap_or("?")
                        ),
                    );
                    d.suggestion = rec.replaced_by.as_ref().map(|k| format!("use `{k}`"));
                    out.push(d);
                }
                Some(_) => {}
            }
        }
    }
}
