//! The search pipeline behind the `seesea` binary.
//!
//! Stages: fan out to sources → fuse → rank on snippets → fetch and extract the best results →
//! rerank on page content → select the sections that answer the query. Every stage after fusion
//! can be skipped or can fail without failing the search; the response says what was degraded.

pub mod cluster;
pub mod content;
pub mod cross;
pub mod error;
pub mod eval;
pub mod fusion;
pub mod journal;
pub mod model;
pub mod pipeline;
pub mod rank;
pub mod render;
pub mod replay;
pub mod report;
pub mod rerank;
pub mod sections;
pub mod semantic;
pub mod learn;
pub mod source;
pub mod store;
pub mod text;

pub use error::{Error, ErrorBody, FailureKind, Result, SourceError, SourceFailure};
pub use model::{Related, 
    Attribution, Content, Hit, Mode, ScoreParts, SearchRequest, SearchResponse, SelectedSection,
    StageTiming, TimeRange,
};
pub use rank::Recency;
pub use journal::Journal;
pub use learn::{LearnConfig, Learner};
pub use pipeline::{host_of, ExtractedPage, Pipeline, PipelineConfig, SourceProbe};
pub use source::{RawHit, Source, SourceQuery};
