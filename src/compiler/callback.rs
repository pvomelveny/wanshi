// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::{HashMap, HashSet};

use crate::slug::Slug;

#[derive(Debug)]
pub struct CallbackValue {
    /// `None` while no parent is known yet. Recording "unknown" distinctly from
    /// a real parent matters because the fallback parent is itself a valid slug
    /// that a section can genuinely be embedded under.
    pub parent: Option<Slug>,
    pub is_parent_specified: bool,

    /// Used to record which sections reference the current section.
    pub backlinks: HashSet<Slug>,

    /// Sections that embed this one, rendering its content inside their own.
    ///
    /// Kept apart from `backlinks` because citing a note and containing it are
    /// different relationships, and a reader usually wants them answered
    /// separately. Kept apart from `parent` because `parent` holds one slug and
    /// an explicitly declared one displaces the embedder entirely — so a note
    /// that is embedded *and* names its own parent used to leave no record of
    /// being embedded at all.
    ///
    /// Direct embedders only, never transitive: if `h` embeds `m` and `m`
    /// embeds `x`, then `x` records `m`.
    pub embedded_by: HashSet<Slug>,
}

/// A section embedded from two places, and the two hosts that claimed it.
///
/// Held rather than reported at the point it is noticed, because at that point
/// the hosts may be sections that will not survive publication — a heading is
/// one — and a warning naming them would name something the author cannot
/// address. See [`Callback::ambiguous_parents`].
#[derive(Debug)]
pub struct AmbiguousParent {
    pub child: Slug,
    pub kept: Slug,
    pub discarded: Slug,
}

/// `.0` is the recorded graph; `.1` is deferred diagnostics, private so nothing
/// reports them before the graph is normalized.
#[derive(Debug)]
pub struct Callback(pub HashMap<Slug, CallbackValue>, Vec<AmbiguousParent>);

impl Callback {
    pub fn new() -> Callback {
        Callback(HashMap::new(), Vec::new())
    }

    pub fn merge(&mut self, other: Callback) {
        let Callback(entries, ambiguous) = other;
        entries.into_iter().for_each(|(s, t)| self.insert(s, t));
        self.1.extend(ambiguous);
    }

    /// The ambiguities noticed while merging, to be reported once the graph is
    /// normalized and the slugs involved are ones the author wrote.
    pub fn take_ambiguous_parents(&mut self) -> Vec<AmbiguousParent> {
        std::mem::take(&mut self.1)
    }

    pub fn insert(&mut self, child_slug: Slug, value: CallbackValue) {
        match self.0.get_mut(&child_slug) {
            None => {
                self.0.insert(child_slug, value);
            }
            Some(existed) => {
                existed.backlinks.extend(value.backlinks);
                existed.embedded_by.extend(value.embedded_by);

                // An explicitly declared parent always wins, and never loses to
                // a later inferred one.
                if existed.is_parent_specified {
                    if value.is_parent_specified && existed.parent != value.parent {
                        color_print::ceprintln!(
                            "<y>Warning: conflicting explicit parents for `{}`; keeping `{}`.</>",
                            child_slug,
                            display_parent(existed.parent),
                        );
                    }
                    return;
                }
                if value.is_parent_specified {
                    existed.parent = value.parent;
                    existed.is_parent_specified = true;
                    return;
                }

                match (existed.parent, value.parent) {
                    // First parent to be inferred wins.
                    (None, incoming) => existed.parent = incoming,
                    (Some(_), None) => {}
                    // Embedding the same section in several places is a
                    // legitimate thing to do — a shared definition, say — but
                    // only one of them can be the parent, and which one is
                    // decided by compilation order rather than by anything the
                    // author intended. Held, not reported: two hosts that both
                    // resolve to the same published section are no ambiguity at
                    // all, and only the normalized graph knows that.
                    (Some(current), Some(incoming)) if current != incoming => {
                        self.1.push(AmbiguousParent {
                            child: child_slug,
                            kept: current,
                            discarded: incoming,
                        });
                    }
                    (Some(_), Some(_)) => {}
                }
            }
        }
    }

    /// Record a parent inferred from an embed.
    pub fn insert_parent(&mut self, child_slug: Slug, parent: Slug) {
        self.insert(
            child_slug,
            CallbackValue {
                parent: Some(parent),
                is_parent_specified: false,
                backlinks: HashSet::new(),
                embedded_by: HashSet::new(),
            },
        );
    }

    /// Record a parent the section declared for itself via `parent` metadata.
    pub fn specify_parent(&mut self, child_slug: Slug, parent: Slug) {
        self.insert(
            child_slug,
            CallbackValue {
                parent: Some(parent),
                is_parent_specified: true,
                backlinks: HashSet::new(),
                embedded_by: HashSet::new(),
            },
        );
    }

    /// Record backlinks without asserting anything about parentage.
    pub fn insert_backlinks<I>(&mut self, child_slug: Slug, backlinks: I)
    where
        I: IntoIterator<Item = Slug>,
    {
        self.insert(
            child_slug,
            CallbackValue {
                parent: None,
                is_parent_specified: false,
                backlinks: HashSet::from_iter(backlinks),
                embedded_by: HashSet::new(),
            },
        );
    }

    /// Record that `host` embeds `child_slug`, without asserting parentage.
    ///
    /// Paired with `insert_parent` at the embed site rather than replacing it:
    /// the parent drives the breadcrumb and can be overridden by the child,
    /// while this is the durable record that the embed happened.
    pub fn insert_embedded_by(&mut self, child_slug: Slug, host: Slug) {
        self.insert(
            child_slug,
            CallbackValue {
                parent: None,
                is_parent_specified: false,
                backlinks: HashSet::new(),
                embedded_by: HashSet::from([host]),
            },
        );
    }
}

fn display_parent(parent: Option<Slug>) -> String {
    parent.map_or_else(|| "<none>".to_string(), |slug| slug.to_string())
}
