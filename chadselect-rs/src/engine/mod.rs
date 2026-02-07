//! Extraction engine modules.
//!
//! Each sub-module handles one query engine and exposes a single `process`
//! function that accepts the engine-specific expression and a [`ContentItem`].

pub mod css;
pub mod json;
pub mod regex;
pub mod xpath;
