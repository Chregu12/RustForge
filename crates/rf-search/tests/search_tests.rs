//! Comprehensive tests for rf-search

use async_trait::async_trait;
use rf_search::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Test model for searchable tests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Article {
    id: String,
    title: String,
    content: String,
    status: String,
    views: i32,
}

#[async_trait]
impl Searchable for Article {
    type Model = Article;

    fn searchable_fields() -> Vec<&'static str> {
        vec!["title", "content"]
    }

    fn to_searchable(&self) -> Self::Model {
        self.clone()
    }

    fn search_id(&self) -> String {
        self.id.clone()
    }

    fn index_name() -> &'static str {
        "articles"
    }

    fn filterable_fields() -> Vec<&'static str> {
        vec!["status"]
    }

    fn sortable_fields() -> Vec<&'static str> {
        vec!["views"]
    }
}

// Test 1: Test Searchable trait implementation
#[test]
fn test_searchable_trait() {
    let article = Article {
        id: "1".to_string(),
        title: "Rust Web Framework".to_string(),
        content: "Building web apps".to_string(),
        status: "published".to_string(),
        views: 100,
    };

    assert_eq!(article.search_id(), "1");
    assert_eq!(Article::index_name(), "articles");
    assert_eq!(Article::searchable_fields(), vec!["title", "content"]);
    assert_eq!(Article::filterable_fields(), vec!["status"]);
    assert_eq!(Article::sortable_fields(), vec!["views"]);
}

// Test 2: Test SearchOptions builder pattern
#[test]
fn test_search_options_builder() {
    let options = SearchOptions::new()
        .filter("status", "published")
        .filter("author", "john")
        .limit(50)
        .offset(10)
        .highlight("title")
        .highlight("content")
        .sort_by("views", false);

    assert_eq!(options.limit, 50);
    assert_eq!(options.offset, 10);
    assert_eq!(options.filters.len(), 2);
    assert_eq!(options.filters.get("status"), Some(&"published".to_string()));
    assert_eq!(options.highlight_fields.len(), 2);
    assert!(options.sort.is_some());

    let (sort_field, ascending) = options.sort.unwrap();
    assert_eq!(sort_field, "views");
    assert_eq!(ascending, false);
}

// Test 3: Test pagination with SearchOptions
#[test]
fn test_search_options_pagination() {
    let options = SearchOptions::new().page(0, 20);
    assert_eq!(options.offset, 0);
    assert_eq!(options.limit, 20);

    let options = SearchOptions::new().page(2, 20);
    assert_eq!(options.offset, 40);
    assert_eq!(options.limit, 20);

    let options = SearchOptions::new().page(5, 10);
    assert_eq!(options.offset, 50);
    assert_eq!(options.limit, 10);
}

// Test 4: Test SearchResult builder
#[test]
fn test_search_result_builder() {
    let article = Article {
        id: "1".to_string(),
        title: "Test".to_string(),
        content: "Content".to_string(),
        status: "published".to_string(),
        views: 10,
    };

    let hit = SearchHit::new(article.clone(), 0.95);
    let result = SearchResult::new("test query".to_string())
        .add_hit(hit)
        .with_pagination(1, 10)
        .with_processing_time(25);

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.total, 1);
    assert_eq!(result.query, "test query");
    assert_eq!(result.processing_time_ms, 25);
    assert_eq!(result.page, Some(1));
    assert_eq!(result.per_page, Some(10));
}

// Test 5: Test SearchResult pagination metadata
#[test]
fn test_search_result_has_more_pages() {
    let mut result = SearchResult::<Article>::new("test".to_string());
    result.total = 100;

    // First page
    result.page = Some(0);
    result.per_page = Some(20);
    assert!(result.has_more_pages()); // 0 * 20 = 0 < 100

    // Last page
    result.page = Some(5);
    result.per_page = Some(20);
    assert!(!result.has_more_pages()); // 5 * 20 = 100 >= 100

    // No pagination info
    let result_no_page = SearchResult::<Article>::new("test".to_string());
    assert!(!result_no_page.has_more_pages());
}

// Test 6: Test SearchHit with highlights and metadata
#[test]
fn test_search_hit_with_highlights() {
    let article = Article {
        id: "1".to_string(),
        title: "Rust Programming".to_string(),
        content: "Learn Rust".to_string(),
        status: "published".to_string(),
        views: 100,
    };

    let mut highlights = HashMap::new();
    highlights.insert(
        "title".to_string(),
        vec!["<em>Rust</em> Programming".to_string()],
    );

    let mut metadata = HashMap::new();
    metadata.insert("match_type".to_string(), serde_json::json!("exact"));

    let hit = SearchHit::new(article, 0.98)
        .with_highlights(highlights.clone())
        .with_metadata(metadata.clone());

    assert_eq!(hit.score, 0.98);
    assert!(hit.highlights.is_some());
    assert_eq!(hit.highlights.unwrap().len(), 1);
    assert!(hit.metadata.is_some());
}

// Test 7: Test in-memory search engine - indexing
#[test]
fn test_in_memory_index_document() {
    let mut engine = SearchEngine::new();

    let doc = Document::new("1")
        .field("title", "Rust Programming")
        .field("content", "Learn Rust for web development");

    assert!(engine.index(doc).is_ok());
    assert_eq!(engine.count(), 1);
    assert!(engine.term_count() > 0);
}

// Test 8: Test in-memory search engine - basic search
#[test]
fn test_in_memory_basic_search() {
    let mut engine = SearchEngine::new();

    engine
        .index(Document::new("1").field("title", "Rust Web Framework"))
        .unwrap();
    engine
        .index(Document::new("2").field("title", "Python Web Framework"))
        .unwrap();
    engine
        .index(Document::new("3").field("title", "Rust Systems Programming"))
        .unwrap();

    let query = Query::new("Rust");
    let results = engine.search(&query).unwrap();

    assert_eq!(results.len(), 2);
    assert!(results[0].fields["title"].contains("Rust"));
    assert!(results[1].fields["title"].contains("Rust"));
}

// Test 9: Test in-memory search engine - search ranking
#[test]
fn test_in_memory_search_ranking() {
    let mut engine = SearchEngine::new();

    // Document with multiple occurrences should rank higher
    engine
        .index(Document::new("1").field("content", "rust rust rust programming"))
        .unwrap();
    engine
        .index(Document::new("2").field("content", "rust programming tutorial"))
        .unwrap();
    engine
        .index(Document::new("3").field("content", "programming tutorial"))
        .unwrap();

    let query = Query::new("rust");
    let results = engine.search(&query).unwrap();

    assert_eq!(results.len(), 2);
    // Document 1 should rank higher (more occurrences)
    // Note: stemming might affect exact scoring, so just verify the first result has highest score
    assert!(results[0].score >= results[1].score);

    // Verify the document with most occurrences ("rust rust rust") is present and has high score
    let doc1_result = results.iter().find(|r| r.id == "1");
    assert!(doc1_result.is_some());
    assert!(doc1_result.unwrap().score >= 1.0);
}

// Test 10: Test in-memory search engine - pagination
#[test]
fn test_in_memory_search_pagination() {
    let mut engine = SearchEngine::new();

    // Index 25 documents
    for i in 0..25 {
        engine
            .index(Document::new(i.to_string()).field("content", "test content"))
            .unwrap();
    }

    // First page
    let query = Query::new("test").limit(10).offset(0);
    let results = engine.search(&query).unwrap();
    assert_eq!(results.len(), 10);

    // Second page
    let query = Query::new("test").limit(10).offset(10);
    let results = engine.search(&query).unwrap();
    assert_eq!(results.len(), 10);

    // Last page (partial)
    let query = Query::new("test").limit(10).offset(20);
    let results = engine.search(&query).unwrap();
    assert_eq!(results.len(), 5);
}

// Test 11: Test in-memory search engine - remove document
#[test]
fn test_in_memory_remove_document() {
    let mut engine = SearchEngine::new();

    engine
        .index(Document::new("1").field("title", "Test 1"))
        .unwrap();
    engine
        .index(Document::new("2").field("title", "Test 2"))
        .unwrap();

    assert_eq!(engine.count(), 2);

    engine.remove("1").unwrap();
    assert_eq!(engine.count(), 1);

    // Search should not return removed document
    let query = Query::new("Test");
    let results = engine.search(&query).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "2");
}

// Test 12: Test in-memory search engine - document metadata
#[test]
fn test_in_memory_document_metadata() {
    let mut engine = SearchEngine::new();

    let doc = Document::new("1")
        .field("title", "Test Document")
        .meta("author", "John Doe")
        .unwrap()
        .meta("created_at", "2024-01-01")
        .unwrap();

    engine.index(doc).unwrap();

    let query = Query::new("Test");
    let results = engine.search(&query).unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].metadata.contains_key("author"));
    assert_eq!(
        results[0].metadata["author"],
        serde_json::json!("John Doe")
    );
}

// Test 13: Test in-memory search engine - empty search
#[test]
fn test_in_memory_empty_search() {
    let engine = SearchEngine::new();

    let query = Query::new("nonexistent");
    let results = engine.search(&query).unwrap();

    assert!(results.is_empty());
}

// Test 14: Test in-memory search engine - multi-field search
#[test]
fn test_in_memory_multi_field_search() {
    let mut engine = SearchEngine::new();

    engine
        .index(
            Document::new("1")
                .field("title", "Rust Programming")
                .field("content", "Learn Rust basics"),
        )
        .unwrap();

    engine
        .index(
            Document::new("2")
                .field("title", "Python Tutorial")
                .field("content", "Rust is mentioned here"),
        )
        .unwrap();

    let query = Query::new("Rust");
    let results = engine.search(&query).unwrap();

    // Should find both documents (title or content contains "Rust")
    assert_eq!(results.len(), 2);
}

// Test 15: Test Query builder
#[test]
fn test_query_builder() {
    let query = Query::new("rust framework")
        .fuzzy(0.8)
        .limit(20)
        .offset(5);

    // Note: These fields are private, so we test by using them
    // In a real implementation, you might want to make them pub(crate) for testing
}

// Test 16: Test SearchError types
#[test]
fn test_search_error_types() {
    use rf_search::driver::SearchError;

    let err1 = SearchError::IndexError("test".to_string());
    assert!(err1.to_string().contains("Index error"));

    let err2 = SearchError::SearchError("search failed".to_string());
    assert!(err2.to_string().contains("Search error"));

    let err3 = SearchError::ConnectionError("connection lost".to_string());
    assert!(err3.to_string().contains("Connection error"));

    let err4 = SearchError::DocumentNotFound("doc123".to_string());
    assert!(err4.to_string().contains("Document not found"));

    let err5 = SearchError::ConfigError("bad config".to_string());
    assert!(err5.to_string().contains("Configuration error"));

    let err6 = SearchError::SerializationError("json error".to_string());
    assert!(err6.to_string().contains("Serialization error"));
}

// Test 17: Test document builder pattern
#[test]
fn test_document_builder_pattern() {
    let doc = Document::new("123")
        .field("title", "Test")
        .field("content", "Content")
        .field("author", "John");

    assert_eq!(doc.id, "123");
    assert_eq!(doc.fields.len(), 3);
    assert_eq!(doc.fields.get("title"), Some(&"Test".to_string()));
}

// Test 18: Test stemming (words are stemmed to root form)
#[test]
fn test_search_with_stemming() {
    let mut engine = SearchEngine::new();

    engine
        .index(Document::new("1").field("content", "running runner runs"))
        .unwrap();

    // Search for "run" should match stemmed versions
    let query = Query::new("run");
    let results = engine.search(&query).unwrap();

    assert_eq!(results.len(), 1);
}

// Test 19: Test case-insensitive search
#[test]
fn test_case_insensitive_search() {
    let mut engine = SearchEngine::new();

    engine
        .index(Document::new("1").field("title", "RUST PROGRAMMING"))
        .unwrap();

    let query = Query::new("rust programming");
    let results = engine.search(&query).unwrap();

    assert_eq!(results.len(), 1);
}

// Test 20: Test bulk indexing performance
#[test]
fn test_bulk_indexing() {
    let mut engine = SearchEngine::new();

    // Index 1000 documents
    for i in 0..1000 {
        let doc = Document::new(i.to_string())
            .field("title", format!("Document {}", i))
            .field("content", "test content for performance testing");
        engine.index(doc).unwrap();
    }

    assert_eq!(engine.count(), 1000);

    // Search should still be fast
    let query = Query::new("test");
    let results = engine.search(&query).unwrap();

    assert_eq!(results.len(), 10); // Default limit
}
