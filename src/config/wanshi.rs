// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use serde::{Deserialize, Serialize};

pub const DEFAULT_SOURCE_DIR: &str = "trees";
pub const DEFAULT_ASSETS_DIR: &str = "assets";
pub const DEFAULT_BASE_URL: &str = "/";

#[derive(Deserialize, Debug, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Wanshi {
    pub trees: String,
    pub assets: String,
    pub base_url: String,
    pub theme_lock: bool,
    pub themes: Vec<String>,
}

impl Default for Wanshi {
    fn default() -> Self {
        Self {
            trees: DEFAULT_SOURCE_DIR.to_string(),
            assets: DEFAULT_ASSETS_DIR.to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
            // wanshi ships one built-in visual identity; lock the (empty by
            // default) theme picker unless the user opts into `themes`.
            theme_lock: true,
            themes: vec![],
        }
    }
}
