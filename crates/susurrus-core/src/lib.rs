//! Susurrus daemon の中核。
//!
//! 役割:
//! - Cernere token verify (起動時)
//! - md store (forums/**/*.md の read/write)
//! - SQLite キャッシュ (../../spec/DB-SCHEMA.md)
//! - Synergos bridge (susurrus-rt 経由)
//! - presence / typing tracker

pub mod config;
pub mod db;
pub mod store;
pub mod cernere;
pub mod presence;

pub use config::Config;
