#![cfg_attr(feature = "docinclude", feature(external_doc))]
#![cfg_attr(feature = "docinclude", doc(include = "../README.md"))]

pub mod config;
mod core;
pub mod metrics;
pub mod raft;
mod replication;


pub use crate::{
    config::{Config, ConfigBuilder, SnapshotPolicy},
    core::State,
    metrics::RaftMetrics,
    raft::Raft,
};

