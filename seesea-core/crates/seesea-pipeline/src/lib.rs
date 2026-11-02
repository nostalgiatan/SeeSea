//! The search pipeline behind the `seesea` binary.
//!
//! Stages: fan out to sources → fuse → rank on snippets → fetch and extract the best results →
//! rerank on page content → select the sections that answer the query. Every stage after fusion
//! can be skipped or can fail without failing the search; the response says what was degraded.

pub mod content;
pub mod error;
pub mod eval;
pub mod fusion;
pub mod model;
pub mod pipeline;
pub mod rank;
pub mod render;
pub mod replay;
pub mod rerank;
pub mod sections;
pub mod source;
pub mod store;
pub mod text;

pub use error::{Error, ErrorBody, FailureKind, Result, SourceError, SourceFailure};
pub use model::{
    Attribution, Content, Hit, Mode, ScoreParts, SearchRequest, SearchResponse, SelectedSection,
    StageTiming, TimeRange,
};
pub use pipeline::{ExtractedPage, Pipeline, PipelineConfig, SourceProbe};
pub use source::{RawHit, Source, SourceQuery};
