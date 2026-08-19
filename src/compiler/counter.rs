// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use std::fmt::Write;

/// Which sequence a section is numbered in.
///
/// Decided by whether the section has a taxon, which is also what decides how
/// its number is rendered — leading the title when there is no taxon, inside the
/// pill when there is. Keeping one rule for both means the number a section
/// shows and the sequence it belongs to can never disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberKind {
    /// A section with no taxon: a Typst heading, or an embedded note that has
    /// no taxon of its own. These form the page's outline.
    Outline,
    /// A section with a taxon — a definition, theorem, remark. Numbered inside
    /// whatever section contains it, the way a paper numbers its statements.
    Statement,
}

/// Where numbering has got to at one level of the section tree.
///
/// A shared prefix and two tallies. Headings and statements advance separately,
/// so the first heading on a page is 1 even when three statements precede it,
/// but both hang off the same prefix, so a statement in section 3 is `3.1` and a
/// subsection of 3 is `3.1` too. The taxon word is what tells those apart, which
/// is the arrangement any paper uses.
#[derive(Debug, Clone)]
pub struct Counter {
    prefix: Vec<u8>,
    outline: u8,
    statement: u8,
}

impl Counter {
    /// The counter a page starts from: no prefix, nothing counted yet.
    pub fn init() -> Self {
        Counter {
            prefix: Vec::new(),
            outline: 0,
            statement: 0,
        }
    }

    /// Take the next number of `kind`.
    ///
    /// Returns its display form and the counter this section's children should
    /// use — prefixed by the number just taken, with both tallies reset, so
    /// numbering inside a section starts again from one.
    pub fn take(&mut self, kind: NumberKind) -> (String, Counter) {
        let tally = match kind {
            NumberKind::Outline => &mut self.outline,
            NumberKind::Statement => &mut self.statement,
        };
        *tally = tally.saturating_add(1);

        let mut path = self.prefix.clone();
        path.push(*tally);

        let mut display = String::new();
        for number in &path {
            let _ = write!(display, "{}.", number);
        }

        (
            display,
            Counter {
                prefix: path,
                outline: 0,
                statement: 0,
            },
        )
    }

    /// The counter children use when this section takes no number of its own.
    ///
    /// An unnumbered section is transparent: what is inside it goes on counting
    /// where the section itself would have, rather than starting a sequence
    /// nothing introduces.
    pub fn passthrough(&self) -> Counter {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(counter: &mut Counter, kind: NumberKind) -> String {
        counter.take(kind).0
    }

    #[test]
    fn test_the_two_sequences_advance_independently() {
        let mut counter = Counter::init();
        assert_eq!(take(&mut counter, NumberKind::Statement), "1.");
        assert_eq!(take(&mut counter, NumberKind::Statement), "2.");
        // The first heading is 1 even though two statements came first.
        assert_eq!(take(&mut counter, NumberKind::Outline), "1.");
        assert_eq!(take(&mut counter, NumberKind::Statement), "3.");
        assert_eq!(take(&mut counter, NumberKind::Outline), "2.");
    }

    #[test]
    fn test_children_are_prefixed_and_start_over() {
        let mut page = Counter::init();
        let (_, _) = page.take(NumberKind::Outline);
        let (_, _) = page.take(NumberKind::Outline);
        let (number, mut inside) = page.take(NumberKind::Outline);
        assert_eq!(number, "3.");
        assert_eq!(take(&mut inside, NumberKind::Statement), "3.1.");
        assert_eq!(take(&mut inside, NumberKind::Statement), "3.2.");
    }

    /// The accepted collision: within one section, a subsection and a statement
    /// can both be `3.1`. This is what LaTeX does, and the taxon word is what
    /// distinguishes them — pinned so it stays a decision rather than a drift.
    #[test]
    fn test_a_subsection_and_a_statement_may_share_a_number() {
        let mut page = Counter::init();
        let (_, mut inside) = page.take(NumberKind::Outline);
        assert_eq!(take(&mut inside, NumberKind::Statement), "1.1.");
        assert_eq!(take(&mut inside, NumberKind::Outline), "1.1.");
    }

    #[test]
    fn test_nesting_deepens_the_prefix() {
        let mut page = Counter::init();
        let (_, mut section) = page.take(NumberKind::Outline);
        let (number, mut sub) = section.take(NumberKind::Outline);
        assert_eq!(number, "1.1.");
        assert_eq!(take(&mut sub, NumberKind::Statement), "1.1.1.");
    }

    /// Closing a section returns to the parent's counter, so a statement written
    /// after a heading picks the top-level sequence back up rather than
    /// restarting or continuing the section's.
    #[test]
    fn test_the_parent_sequence_survives_a_section() {
        let mut page = Counter::init();
        assert_eq!(take(&mut page, NumberKind::Statement), "1.");
        let (_, mut inside) = page.take(NumberKind::Outline);
        assert_eq!(take(&mut inside, NumberKind::Statement), "1.1.");
        assert_eq!(take(&mut page, NumberKind::Statement), "2.");
    }

    #[test]
    fn test_passthrough_leaves_an_unnumbered_section_transparent() {
        let mut page = Counter::init();
        assert_eq!(take(&mut page, NumberKind::Statement), "1.");
        let mut inherited = page.passthrough();
        assert_eq!(take(&mut inherited, NumberKind::Statement), "2.");
    }
}
