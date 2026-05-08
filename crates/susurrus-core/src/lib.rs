//! Susurrus daemon の中核。
//!
//! 役割:
//! - Cernere token verify (起動時)
//! - md store (forums/**/*.md の read/write)
//! - SQLite キャッシュ (../../spec/DB-SCHEMA.md)
//! - Synergos bridge (susurrus-rt 経由)
//! - presence / typing tracker

pub mod cernere;
pub mod compose;
pub mod config;
pub mod db;
pub mod indexer;
pub mod presence;
pub mod query;
pub mod store;
pub mod text;

pub use config::Config;
pub use db::Db;
