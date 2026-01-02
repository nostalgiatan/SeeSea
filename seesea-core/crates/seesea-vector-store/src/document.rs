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
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/// Document structure for vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Document ID
    pub id: String,

    /// Document content
    pub content: String,

    /// Document title
    pub title: String,

    /// Document URL
    pub url: String,

    /// Document summary
    pub summary: Option<String>,

    /// Document embedding (optional)
    pub embedding: Option<Vec<f32>>,

    /// Document metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Document hash (for change detection)
    pub content_hash: String,

    /// Creation timestamp
    pub created_at: u64,

    /// Update timestamp
    pub updated_at: u64,
}

impl Document {
    /// Create a new document
    pub fn new(
        content: String,
        title: String,
        url: String,
        summary: Option<String>,
        embedding: Option<Vec<f32>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        let content_hash = Self::compute_content_hash(&content);
        let now = Self::current_timestamp();

        Self {
            id: Uuid::new_v4().to_string(),
            content,
            title,
            url,
            summary,
            embedding,
            metadata: metadata.unwrap_or_default(),
            content_hash,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new document with custom ID
    pub fn new_with_id(
        id: String,
        content: String,
        title: String,
        url: String,
        summary: Option<String>,
        embedding: Option<Vec<f32>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        let content_hash = Self::compute_content_hash(&content);
        let now = Self::current_timestamp();

        Self {
            id,
            content,
            title,
            url,
            summary,
            embedding,
            metadata: metadata.unwrap_or_default(),
            content_hash,
            created_at: now,
            updated_at: now,
        }
    }

    /// Compute content hash using SHA-256
    pub fn compute_content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get current timestamp in milliseconds
    pub fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64
    }

    /// Update document content
    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.content_hash = Self::compute_content_hash(&self.content);
        self.updated_at = Self::current_timestamp();
    }

    /// Update document embedding
    pub fn update_embedding(&mut self, embedding: Vec<f32>) {
        self.embedding = Some(embedding);
        self.updated_at = Self::current_timestamp();
    }

    /// Check if content has changed
    pub fn content_changed(&self, new_content: &str) -> bool {
        self.content_hash != Self::compute_content_hash(new_content)
    }

    /// Convert to JSON representation
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Create from JSON representation
    pub fn from_json(json: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(json)
    }
}
