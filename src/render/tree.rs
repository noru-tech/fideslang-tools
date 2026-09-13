//! Terminal tree rendering with box-drawing (or ASCII) guides.

use std::io::{self, Write};

/// A node to draw. Labels may already contain ANSI styling.
#[derive(Debug, Clone, Default)]
pub struct TreeNode {
    pub label: String,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(label: impl Into<String>) -> Self {
        TreeNode {
            label: label.into(),
            children: Vec::new(),
        }
    }

    pub fn with_children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    /// Number of nodes in the subtree, including this one.
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(TreeNode::count).sum::<usize>()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Guides {
    #[default]
    Unicode,
    Ascii,
}

struct Glyphs {
    branch: &'static str,
    last: &'static str,
    vertical: &'static str,
    blank: &'static str,
}

impl Guides {
    fn glyphs(self) -> Glyphs {
        match self {
            Guides::Unicode => Glyphs {
                branch: "├── ",
                last: "└── ",
                vertical: "│   ",
                blank: "    ",
            },
            Guides::Ascii => Glyphs {
                branch: "|-- ",
                last: "`-- ",
                vertical: "|   ",
                blank: "    ",
            },
        }
    }
}

/// Render `roots` as a forest. Each root starts at column 0.
pub fn render(w: &mut dyn Write, roots: &[TreeNode], guides: Guides) -> io::Result<()> {
    let g = guides.glyphs();
    for root in roots {
        writeln!(w, "{}", root.label)?;
        render_children(w, &root.children, "", &g)?;
    }
    Ok(())
}

fn render_children(
    w: &mut dyn Write,
    children: &[TreeNode],
    prefix: &str,
    g: &Glyphs,
) -> io::Result<()> {
    let n = children.len();
    for (i, child) in children.iter().enumerate() {
        let last = i + 1 == n;
        writeln!(
            w,
            "{prefix}{}{}",
            if last { g.last } else { g.branch },
            child.label
        )?;
        let next = format!("{prefix}{}", if last { g.blank } else { g.vertical });
        render_children(w, &child.children, &next, g)?;
    }
    Ok(())
}

/// Render to a `String` (tests, snapshots).
pub fn to_string(roots: &[TreeNode], guides: Guides) -> String {
    let mut buf = Vec::new();
    render(&mut buf, roots, guides).expect("writing to Vec");
    String::from_utf8(buf).expect("utf-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_guides() {
        let tree = TreeNode::new("root").with_children(vec![
            TreeNode::new("a").with_children(vec![TreeNode::new("a1")]),
            TreeNode::new("b"),
        ]);
        let s = to_string(&[tree], Guides::Unicode);
        assert_eq!(s, "root\n├── a\n│   └── a1\n└── b\n");
        let s = to_string(
            &[TreeNode::new("x").with_children(vec![TreeNode::new("y")])],
            Guides::Ascii,
        );
        assert_eq!(s, "x\n`-- y\n");
    }
}
