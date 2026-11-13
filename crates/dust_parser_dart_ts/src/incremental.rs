use std::{cell::RefCell, collections::VecDeque};

use tree_sitter::{InputEdit, Parser, Point, Tree};

/// Each parser worker retains at most four immutable native trees.
const MAX_TREES: usize = 4;
/// Large dependency files never enter the retained input cache.
const MAX_BYTES: usize = 8 * 1024;
/// Dense small files must also fit the native syntax retention bound.
const MAX_NODES: usize = 4096;
/// Unrelated files take a fresh parse instead of applying many small edits.
const MAX_EDITS: usize = 32;

thread_local! {
    /// Only native syntax survives; every caller extracts facts from its current input.
    static TREES: RefCell<Cache> = RefCell::default();
}

/// Native reuse is independent of source IDs, language gates and extraction mode.
pub(super) fn parse(parser: &mut Parser, source: &str) -> Option<Tree> {
    if source.len() > MAX_BYTES {
        return parser.parse(source, None);
    }
    TREES.with(|cache| cache.borrow_mut().parse(parser, source))
}

/// Native syntax candidates retained only within the current parser thread.
#[derive(Default)]
struct Cache {
    /// Most recently used small valid inputs come first.
    entries: VecDeque<Entry>,
}

/// A tree and the exact original bytes needed to describe future native edits.
struct Entry {
    /// Original UTF-8 text, bounded independently of its native node count.
    source: String,
    /// Inclusive LF splitting keeps row-preserving candidates together.
    lines: usize,
    /// Always a successfully parsed tree without syntax errors.
    tree: Tree,
}

impl Cache {
    /// Reuses a nearby small input while every returned error still comes from a fresh parse.
    fn parse(&mut self, parser: &mut Parser, source: &str) -> Option<Tree> {
        let lines = source.split_inclusive('\n').count();
        let candidate = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.lines == lines)
            .min_by_key(|(_, entry)| {
                (
                    entry.source != source,
                    entry.source.len().abs_diff(source.len()),
                )
            })
            .map(|(index, _)| index);
        if let Some(index) = candidate
            && self.entries[index].source == source
        {
            let entry = self.entries.remove(index).expect("selected native tree");
            let tree = entry.tree.clone();
            self.entries.push_front(entry);
            return Some(tree);
        }
        let edited = candidate.and_then(|index| {
            let entry = &self.entries[index];
            let edits = line_edits(&entry.source, source)?;
            let mut tree = entry.tree.clone();
            for edit in edits.iter().rev() {
                tree.edit(edit);
            }
            Some(tree)
        });
        let tree = if let Some(previous) = &edited {
            match parser.parse(source, Some(previous)) {
                Some(tree) if !tree.root_node().has_error() => Some(tree),
                _ => {
                    // A cancelled parse must not resume while taking the fresh fallback.
                    parser.reset();
                    parser.parse(source, None)
                }
            }
        } else {
            parser.parse(source, None)
        }?;
        if !tree.root_node().has_error() && tree.root_node().descendant_count() <= MAX_NODES {
            if edited.is_some() {
                self.entries.remove(candidate.expect("edited native tree"));
            }
            self.entries.push_front(Entry {
                source: source.to_owned(),
                lines,
                tree: tree.clone(),
            });
            self.entries.truncate(MAX_TREES);
        }
        Some(tree)
    }
}

/// Line-count and whole-line change budgets bound reuse before trimming edits.
fn line_edits(old: &str, new: &str) -> Option<Vec<InputEdit>> {
    let mut old_lines = old.split_inclusive('\n');
    let mut new_lines = new.split_inclusive('\n');
    let mut edits = Vec::new();
    let (mut row, mut offset, mut old_changed, mut new_changed) = (0, 0, 0, 0);
    loop {
        let (old_line, new_line) = match (old_lines.next(), new_lines.next()) {
            (None, None) => break,
            (Some(old), Some(new)) => (old, new),
            _ => return None,
        };
        if old_line != new_line {
            let edit = line_edit(old_line, new_line, row, offset);
            old_changed += edit.old_end_byte - edit.start_byte;
            new_changed += edit.new_end_byte - edit.start_byte;
            if edits.len() == MAX_EDITS
                || old_changed.max(new_changed) > old.len().max(new.len()) / 2
            {
                return None;
            }
            edits.push(edit);
        }
        offset += old_line.len();
        row += 1;
    }
    Some(edits)
}

/// Equal UTF-8 prefixes and suffixes stay outside the edited native syntax range.
fn line_edit(old: &str, new: &str, row: usize, offset: usize) -> InputEdit {
    let mut prefix = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(prefix) || !new.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let mut suffix = old.as_bytes()[prefix..]
        .iter()
        .rev()
        .zip(new.as_bytes()[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(old.len() - suffix) || !new.is_char_boundary(new.len() - suffix) {
        suffix -= 1;
    }
    let old_end = old.len() - suffix;
    let new_end = new.len() - suffix;
    InputEdit {
        start_byte: offset + prefix,
        old_end_byte: offset + old_end,
        new_end_byte: offset + new_end,
        start_position: line_point(row, old, prefix),
        old_end_position: line_point(row, old, old_end),
        new_end_position: line_point(row, new, new_end),
    }
}

/// Tree-sitter columns count bytes; only an included LF advances the row.
fn line_point(row: usize, line: &str, offset: usize) -> Point {
    if offset == line.len() && line.ends_with('\n') {
        Point::new(row + 1, 0)
    } else {
        Point::new(row, offset)
    }
}

#[cfg(test)]
mod tests;
