//! 宏测试

use seesea_derive::EngineType::General;
use seesea_derive::*;
use seesea_errors::{empty_field, invalid_format, out_of_range};

#[test]
fn test_engine_info_macro() {
    // 测试engine_info!宏
    let metadata = engine_info! {
        name: "TestEngine",
        engine_type: General,
        website: "https://test.example.com",
        categories: ["general", "web"],
        max_page_size: 10,
        supports_pagination: true,
        supports_time_range: false,
        supports_language_filter: true,
        supports_region_filter: false,
        supports_safe_search: true,
        shortcut: None
    };

    assert_eq!(metadata.name, "TestEngine");
    assert_eq!(metadata.categories, vec!["general", "web"]);
    assert!(metadata.capabilities.supports_pagination);
    assert!(!metadata.capabilities.supports_time_range);
    assert!(metadata.capabilities.supports_safe_search);

    let about = metadata.about;
    assert_eq!(about.website, Some("https://test.example.com".to_string()));
    assert_eq!(about.wikidata_id, None);
    assert!(!about.use_official_api);
    assert!(!about.require_api_key);
    assert_eq!(about.results, "HTML");
}

#[test]
fn test_engine_info_macro_minimal() {
    // 测试最小化的engine_info!宏
    let metadata = engine_info! {
        name: "MinimalEngine",
        engine_type: General,
        website: "https://minimal.example.com",
        categories: ["general"],
        max_page_size: 5,
        supports_pagination: false,
        supports_time_range: false,
        supports_language_filter: false,
        supports_region_filter: false,
        supports_safe_search: false,
        shortcut: None
    };

    assert_eq!(metadata.name, "MinimalEngine");
    assert_eq!(metadata.categories, vec!["general"]);
    assert!(!metadata.capabilities.supports_pagination);
    assert!(!metadata.capabilities.supports_time_range);
    assert!(!metadata.capabilities.supports_safe_search);
}

#[test]
fn test_engine_metadata_macro_with_api() {
    // 测试需要API密钥的引擎元数据
    let metadata = engine_metadata! {
        name: "APIEngine",
        categories: ["general", "academic"],
        paging: true,
        time_range_support: true,
        safesearch: true,
        about: {
            website: "https://api.example.com",
            wikidata_id: "QAPI456",
            use_official_api: true,
            require_api_key: true,
            results: "JSON"
        }
    };

    assert_eq!(metadata.name, "APIEngine");
    assert_eq!(metadata.categories, vec!["general", "academic"]);
    assert!(metadata.paging);
    assert!(metadata.time_range_support);
    assert!(metadata.safesearch);

    let about = metadata.about.unwrap();
    assert_eq!(about.website, Some("https://api.example.com".to_string()));
    assert_eq!(about.wikidata_id, Some("QAPI456".to_string()));
    assert!(about.use_official_api);
    assert!(about.require_api_key);
    assert_eq!(about.results, "JSON");
}

#[test]
fn test_engine_metadata_macro_specialized() {
    // 测试专业搜索引擎的元数据
    let metadata = engine_metadata! {
        name: "ImageSearchEngine",
        categories: ["image", "visual"],
        paging: true,
        time_range_support: false,
        safesearch: true,
        about: {
            website: "https://images.example.com",
            wikidata_id: "QImage789",
            use_official_api: true,
            require_api_key: false,
            results: "JSON+Images"
        }
    };

    assert_eq!(metadata.name, "ImageSearchEngine");
    assert_eq!(metadata.categories, vec!["image", "visual"]);
    assert!(metadata.paging);
    assert!(!metadata.time_range_support);
    assert!(metadata.safesearch);

    let about = metadata.about.unwrap();
    assert_eq!(
        about.website,
        Some("https://images.example.com".to_string())
    );
    assert_eq!(about.wikidata_id, Some("QImage789".to_string()));
    assert!(about.use_official_api);
    assert!(!about.require_api_key);
    assert_eq!(about.results, "JSON+Images");
}

#[test]
fn test_error_creation_macros() {
    // 测试错误创建宏
    let empty_field_error = empty_field("username");
    assert_eq!(empty_field_error.code(), 4001); // EMPTY_FIELD = 4001
    assert!(empty_field_error.message().contains("username"));
    assert!(empty_field_error.message().contains("字段"));

    let invalid_format_error = invalid_format("email");
    assert_eq!(invalid_format_error.code(), 3006); // INVALID_FORMAT = 3006 (PARSE_ERROR_BASE + 6)
    assert!(invalid_format_error.message().contains("email"));

    let out_of_range_error = out_of_range("age", 1, 100, 50);
    assert_eq!(out_of_range_error.code(), 4011); // OUT_OF_RANGE = 4011
    assert!(out_of_range_error.message().contains("age"));
    assert!(out_of_range_error.message().contains("1"));
    assert!(out_of_range_error.message().contains("100"));
}

#[test]
fn test_error_creation_macro_with_custom_message() {
    // 测试自定义消息的错误创建宏
    let custom_error = ValidationError {
        field: Some("custom_field".to_string()),
        message: "Custom error message".to_string(),
        code: "CUSTOM_ERROR".to_string(),
    };

    assert_eq!(custom_error.field, Some("custom_field".to_string()));
    assert_eq!(custom_error.message, "Custom error message");
    assert_eq!(custom_error.code, "CUSTOM_ERROR");
}

#[test]
fn test_error_message_formatting() {
    // 测试错误消息格式化
    let empty_field_error = empty_field("query");
    assert!(empty_field_error.message().contains("query"));
    assert!(empty_field_error.message().contains("不能为空"));

    let out_of_range_error = out_of_range("page_size", 1, 100, 50);
    assert!(out_of_range_error.message().contains("page_size"));
    assert!(out_of_range_error.message().contains("1"));
    assert!(out_of_range_error.message().contains("100"));
    assert!(out_of_range_error.message().contains("之间"));
}

#[test]
fn test_macro_generated_code_validity() {
    // 测试宏生成的代码是否有效
    let metadata = engine_metadata! {
        name: "ValidEngine",
        categories: ["test"],
        paging: true,
        time_range_support: false,
        safesearch: true,
        about: {
            website: "https://valid.example.com",
            wikidata_id: "QValid123",
            use_official_api: true,
            require_api_key: false,
            results: "XML"
        }
    };

    // 验证所有字段都可以正确访问
    assert!(!metadata.name.is_empty());
    assert!(!metadata.categories.is_empty());
    assert!(metadata.about.is_some());

    let about = metadata.about.unwrap();
    assert!(about.website.is_some());
    assert!(!about.website.as_ref().unwrap().is_empty());
    assert!(about.wikidata_id.is_some());
    assert!(!about.wikidata_id.as_ref().unwrap().is_empty());
    assert!(!about.results.is_empty());
}

#[test]
fn test_macro_with_empty_categories() {
    // 测试空分类的宏
    let metadata = engine_metadata! {
        name: "EmptyCategoriesEngine",
        categories: [],
        paging: false,
        time_range_support: false,
        safesearch: false
    };

    assert_eq!(metadata.name, "EmptyCategoriesEngine");
    assert!(metadata.categories.is_empty());
    assert!(!metadata.paging);
    assert!(!metadata.time_range_support);
    assert!(!metadata.safesearch);
    assert!(metadata.about.is_none());
}

#[test]
fn test_macro_with_multiple_categories() {
    // 测试多分类的宏
    let metadata = engine_metadata! {
        name: "MultiCategoriesEngine",
        categories: ["general", "academic", "code", "documentation"],
        paging: true,
        time_range_support: true,
        safesearch: true,
        about: {
            website: "https://multi.example.com",
            wikidata_id: "QMulti456",
            use_official_api: false,
            require_api_key: false,
            results: "HTML"
        }
    };

    assert_eq!(metadata.name, "MultiCategoriesEngine");
    assert_eq!(metadata.categories.len(), 4);
    assert!(metadata.categories.contains(&"general".to_string()));
    assert!(metadata.categories.contains(&"academic".to_string()));
    assert!(metadata.categories.contains(&"code".to_string()));
    assert!(metadata.categories.contains(&"documentation".to_string()));
}
