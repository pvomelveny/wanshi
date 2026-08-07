// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

mod core;
mod document;
mod header;

pub use core::{
    catalog_item, html_article_inner, html_catalog_block, html_footer, html_footer_section,
    html_header_nav, html_link, html_query_block, html_query_item, CatalogItemArgs,
};
pub use document::{html_doc, html_main_script, html_main_style};
pub use header::{html_header, HtmlHeaderArgs};
