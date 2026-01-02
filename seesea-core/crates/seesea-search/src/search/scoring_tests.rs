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

//! Tests for the scoring system
//!
//! This module tests the BM25 scoring algorithm and score preservation

use super::*;
use seesea_derive::{ResultType, SearchQuery, SearchResultItem};
use std::collections::HashMap;

/// Helper function to create a test result item
fn create_test_item(title: &str, content: &str, url: &str) -> SearchResultItem {
    SearchResultItem {
        title: title.to_string(),
        url: url.to_string(),
        content: content.to_string(),
        display_url: None,
        site_name: None,
        score: 1.0,
        result_type: ResultType::Web,
        thumbnail: None,
        published_date: None,
        template: None,
        metadata: HashMap::new(),
    }
}

#[test]
fn test_tokenize_basic() {
    let tokens = tokenize("Hello world");
    assert_eq!(tokens, vec!["hello", "world"]);
}

#[test]
fn test_tokenize_with_punctuation() {
    let tokens = tokenize("Hello, world! This is a test.");
    assert_eq!(tokens, vec!["hello", "world", "this", "is", "a", "test"]);
}

#[test]
fn test_tokenize_empty_string() {
    let tokens = tokenize("");
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_bm25_score_exact_match() {
    let params = BM25Params::default();
    let score = bm25_score("rust programming", "rust programming", 2.0, &params);
    assert!(score > 0.0, "Score should be positive for exact match");
    assert!(score <= 1.0, "Score should be normalized to <= 1.0");
}

#[test]
fn test_bm25_score_partial_match() {
    let params = BM25Params::default();
    let score = bm25_score("rust programming language tutorial", "rust", 2.0, &params);
    assert!(score > 0.0, "Score should be positive for partial match");
}

#[test]
fn test_bm25_score_no_match() {
    let params = BM25Params::default();
    let score = bm25_score("python programming", "rust", 2.0, &params);
    assert_eq!(score, 0.0, "Score should be 0 for no match");
}

#[test]
fn test_bm25_score_multiple_occurrences() {
    let params = BM25Params::default();
    let score1 = bm25_score("rust", "rust", 1.0, &params);
    let score2 = bm25_score("rust rust rust", "rust", 3.0, &params);
    assert!(score2 > score1, "Multiple occurrences should score higher");
}

#[test]
fn test_exact_match_bonus_perfect_match() {
    let bonus = exact_match_bonus("rust programming", "rust programming");
    assert_eq!(bonus, 1.0, "Perfect match should get bonus of 1.0");
}

#[test]
fn test_exact_match_bonus_starts_with() {
    let bonus = exact_match_bonus("rust programming language", "rust");
    assert_eq!(bonus, 0.8, "Starts with match should get bonus of 0.8");
}

#[test]
fn test_exact_match_bonus_no_match() {
    let bonus = exact_match_bonus("python", "rust");
    assert_eq!(bonus, 0.0, "No match should get bonus of 0.0");
}

#[test]
fn test_url_relevance_match() {
    let score = url_relevance("https://www.rust-lang.org/", "rust");
    assert!(
        score > 0.0,
        "URL containing query term should have positive score"
    );
}

#[test]
fn test_url_relevance_no_match() {
    let score = url_relevance("https://www.python.org/", "rust");
    assert_eq!(
        score, 0.0,
        "URL not containing query term should have zero score"
    );
}

#[test]
fn test_position_score_decreases() {
    let score0 = position_score(0);
    let score5 = position_score(5);
    let score10 = position_score(10);

    assert!(score0 > score5, "Earlier positions should score higher");
    assert!(score5 > score10, "Earlier positions should score higher");
}

#[test]
fn test_position_score_range() {
    let score = position_score(0);
    assert!(
        score > 0.0 && score <= 1.0,
        "Position score should be in (0, 1]"
    );
}

#[test]
fn test_engine_authority_google() {
    let score = get_engine_authority("google");
    assert_eq!(score, 1.0, "Google should have maximum authority");
}

#[test]
fn test_engine_authority_unknown() {
    let score = get_engine_authority("unknown_engine");
    assert_eq!(score, 0.70, "Unknown engines should have default authority");
}

#[test]
fn test_calculate_score_returns_valid_range() {
    let item = create_test_item(
        "Rust Programming Language",
        "Rust is a systems programming language",
        "https://www.rust-lang.org/",
    );
    let query = SearchQuery {
        query: "rust programming".to_string(),
        page: 1,
        page_size: 10,
        ..Default::default()
    };

    let score = calculate_score(&item, &query, "google", 0, &ScoringContext::default());

    assert!(
        (0.0..=1.0).contains(&score),
        "Score should be in [0, 1] range, got {score}"
    );
}

#[test]
fn test_calculate_score_title_matters() {
    let item1 = create_test_item("Rust Programming", "content", "https://example.com/");
    let item2 = create_test_item("Unrelated Title", "content", "https://example.com/");
    let query = SearchQuery {
        query: "rust programming".to_string(),
        page: 1,
        page_size: 10,
        ..Default::default()
    };

    let score1 = calculate_score(&item1, &query, "google", 0, &ScoringContext::default());
    let score2 = calculate_score(&item2, &query, "google", 0, &ScoringContext::default());

    assert!(score1 > score2, "Relevant title should score higher");
}

#[test]
fn test_score_results_modifies_scores() {
    let mut items = vec![
        create_test_item("Rust Programming", "Learn Rust", "https://rust-lang.org/"),
        create_test_item("Python Guide", "Learn Python", "https://python.org/"),
    ];

    let query = SearchQuery {
        query: "rust programming".to_string(),
        page: 1,
        page_size: 10,
        ..Default::default()
    };

    // Initial scores should be 1.0
    assert_eq!(items[0].score, 1.0);
    assert_eq!(items[1].score, 1.0);

    score_results(&mut items, &query, "google", None, None);

    // Scores should be different after scoring
    assert_ne!(items[0].score, 1.0, "First item score should change");
    assert_ne!(items[1].score, 1.0, "Second item score should change");
    assert!(
        items[0].score > items[1].score,
        "Rust item should score higher for rust query"
    );
}

#[test]
fn test_score_and_sort_results() {
    let mut items = vec![
        create_test_item("Python Guide", "Learn Python", "https://python.org/"),
        create_test_item("Rust Programming", "Learn Rust", "https://rust-lang.org/"),
        create_test_item("Rust Tutorial", "Advanced Rust", "https://example.com/rust"),
    ];

    let query = SearchQuery {
        query: "rust programming".to_string(),
        page: 1,
        page_size: 10,
        ..Default::default()
    };

    score_and_sort_results(&mut items, &query, "google", None);

    // After sorting, rust items should be first
    assert!(
        items[0].title.to_lowercase().contains("rust"),
        "First item should be about rust"
    );
    assert!(
        items[0].score >= items[1].score,
        "Items should be sorted by score descending"
    );
    assert!(
        items[1].score >= items[2].score,
        "Items should be sorted by score descending"
    );
}

#[test]
fn test_scoring_weights_default() {
    let weights = ScoringWeights::default();

    // Verify weights sum to approximately 1.0
    let sum = weights.title_bm25
        + weights.content_bm25
        + weights.url_match
        + weights.engine_authority
        + weights.position_weight;
    assert!((sum - 1.0).abs() < 0.01, "Weights should sum to ~1.0");
}

#[test]
fn test_bm25_params_default() {
    let params = BM25Params::default();

    assert_eq!(params.k1, 1.5, "Default k1 should be 1.5");
    assert_eq!(params.b, 0.75, "Default b should be 0.75");
}

#[test]
fn test_empty_items_scoring() {
    let mut items: Vec<SearchResultItem> = vec![];
    let query = SearchQuery::default();

    // Should not panic
    score_results(&mut items, &query, "google", None, None);
    assert_eq!(items.len(), 0);
}
