// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use serde::{Deserialize, Serialize};

pub use seesea_config::DynamicAdjustmentConfig;

/// Vector store result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreResult {
    /// Document ID
    pub id: String,

    /// Score (similarity)
    pub score: f32,

    /// Document payload
    pub payload: serde_json::Value,
}

/// Vector store statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreStats {
    /// Number of points in the collection
    pub points_count: usize,

    /// Number of vectors in the collection
    pub vectors_count: usize,

    /// Collection size in bytes
    pub collection_size: u64,

    /// Dimension of vectors
    pub dimension: usize,

    /// Distance metric
    pub distance: String,

    /// Number of shards
    pub shard_number: usize,

    /// Replication factor
    pub replication_factor: usize,
}
