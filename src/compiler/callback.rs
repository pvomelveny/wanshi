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
}

#[derive(Debug)]
pub struct Callback(pub HashMap<Slug, CallbackValue>);

impl Callback {
    pub fn new() -> Callback {
        Callback(HashMap::new())
    }

    pub fn merge(&mut self, other: Callback) {
        other.0.into_iter().for_each(|(s, t)| self.insert(s, t));
    }

    pub fn insert(&mut self, child_slug: Slug, value: CallbackValue) {
        match self.0.get_mut(&child_slug) {
            None => {
                self.0.insert(child_slug, value);
            }
            Some(existed) => {
                existed.backlinks.extend(value.backlinks);

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
                    (Some(current), Some(incoming)) if current != incoming => {
                        color_print::ceprintln!(
                            "<y>Warning: Multiple parents for `{}`: `{}` and `{}`. Using {}.</>",
                            child_slug,
                            current,
                            incoming,
                            current
                        );
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
            },
        );
    }
}

fn display_parent(parent: Option<Slug>) -> String {
    parent.map_or_else(|| "<none>".to_string(), |slug| slug.to_string())
}
