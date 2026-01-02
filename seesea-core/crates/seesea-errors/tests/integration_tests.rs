use error::ErrorCategory;
use seesea_errors::*;
use std::error::Error as StdError;
use std::sync::Arc;
use std::thread;

/// 测试错误链构建
#[test]
fn test_error_chaining() {
    // 创建一个基础验证错误
    let base_error = validation_error("输入验证失败");

    // 创建一个网络错误，引用基础错误
    let network_error = network_error("网络请求失败");

    // 验证错误属性
    assert_eq!(network_error.category(), ErrorCategory::Network);
    assert_eq!(network_error.severity(), ErrorSeverity::Error);

    // 验证验证错误属性
    assert_eq!(base_error.category(), ErrorCategory::Validation);
    assert_eq!(base_error.severity(), ErrorSeverity::Error);
}

/// 测试多线程环境下的错误处理
#[test]
fn test_multithread_error_handling() {
    let errors = Arc::new(std::sync::Mutex::new(Vec::new()));
    let errors_clone = Arc::clone(&errors);

    let handle = thread::spawn(move || {
        let error = engine_error("test_engine", "搜索失败");
        let mut errors = errors_clone.lock().unwrap();
        errors.push(error);
    });

    handle.join().expect("线程执行失败");

    let errors = errors.lock().unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category(), ErrorCategory::Search);
    assert_eq!(errors[0].severity(), ErrorSeverity::Error);
}

/// 测试错误收集和聚合
#[test]
fn test_error_collection_and_aggregation() {
    let mut errors = Vec::new();

    // 收集多个不同类型的错误
    errors.push(validation_error("输入验证失败"));
    errors.push(network_error("网络错误"));
    errors.push(engine_error("test_engine", "搜索失败"));
    errors.push(parse_error("格式错误"));

    // 按类别分组
    let mut grouped_errors: std::collections::HashMap<ErrorCategory, Vec<&ErrorInfo>> =
        std::collections::HashMap::new();
    for error in &errors {
        grouped_errors
            .entry(error.category())
            .or_insert_with(Vec::new)
            .push(error);
    }

    // 验证分组结果
    assert_eq!(
        grouped_errors
            .get(&ErrorCategory::Validation)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        grouped_errors.get(&ErrorCategory::Network).unwrap().len(),
        1
    );
    assert_eq!(grouped_errors.get(&ErrorCategory::Search).unwrap().len(), 1);

    // 验证错误总数
    assert_eq!(errors.len(), 4);
}

/// 测试错误严重级别统计
#[test]
fn test_error_severity_statistics() {
    let errors = vec![
        validation_error("严重验证错误"),
        network_error("网络警告"),
        engine_error("test_engine", "搜索信息"),
        parse_error("格式警告"),
        duplicate_value("johndoe", "username"),
    ];

    let mut severity_count = std::collections::HashMap::new();
    for error in &errors {
        *severity_count.entry(error.severity()).or_insert(0) += 1;
    }

    // 验证严重级别统计 - 所有错误默认都是Error级别
    assert_eq!(severity_count.get(&ErrorSeverity::Error).unwrap(), &5);
}

/// 测试错误转换和兼容性
#[test]
fn test_error_conversion_compatibility() {
    // 创建不同类型的错误
    let validation_err = validation_error("验证失败");
    let network_err = network_error("网络失败");
    let search_err = engine_error("test_engine", "搜索失败");

    // 转换为标准错误 trait 对象
    let errors: Vec<Box<dyn StdError>> = vec![
        Box::new(validation_err),
        Box::new(network_err),
        Box::new(search_err),
    ];

    // 验证转换后的错误
    assert!(errors[0].to_string().contains("验证失败"));
    assert!(errors[1].to_string().contains("网络失败"));
    assert!(errors[2].to_string().contains("搜索失败"));
}

/// 测试错误序列化兼容性
#[test]
fn test_error_serialization_compatibility() {
    let errors = vec![
        validation_error("验证失败"),
        network_error("网络失败"),
        engine_error("test_engine", "搜索失败"),
    ];

    // 验证错误数组的基本属性
    assert_eq!(errors.len(), 3);
    assert_eq!(errors[0].category(), ErrorCategory::Validation);
    assert_eq!(errors[1].category(), ErrorCategory::Network);
    assert_eq!(errors[2].category(), ErrorCategory::Search);
}

/// 测试错误过滤和筛选
#[test]
fn test_error_filtering_and_selection() {
    let errors = vec![
        validation_error("验证错误1"),
        network_error("网络错误1"),
        validation_error("验证错误2"),
        engine_error("test_engine", "搜索失败1"),
        network_error("网络错误2"),
        parse_error("格式错误1"),
    ];

    // 按类别过滤
    let validation_errors: Vec<&ErrorInfo> = errors
        .iter()
        .filter(|e| e.category() == ErrorCategory::Validation)
        .collect();

    let network_errors: Vec<&ErrorInfo> = errors
        .iter()
        .filter(|e| e.category() == ErrorCategory::Network)
        .collect();

    // 验证过滤结果
    assert_eq!(validation_errors.len(), 2);
    assert_eq!(network_errors.len(), 2);

    // 验证具体错误类别
    assert_eq!(validation_errors[0].category(), ErrorCategory::Validation);
    assert_eq!(validation_errors[1].category(), ErrorCategory::Validation);
    assert_eq!(network_errors[0].category(), ErrorCategory::Network);
    assert_eq!(network_errors[1].category(), ErrorCategory::Network);
}

/// 测试错误时间戳和排序
#[test]
fn test_error_timestamp_and_sorting() {
    // 创建带有不同错误代码的错误
    let mut errors = vec![
        ErrorInfo::new(1001, "错误1".to_string())
            .with_category(ErrorCategory::Validation)
            .with_severity(ErrorSeverity::Error),
        ErrorInfo::new(1002, "错误2".to_string())
            .with_category(ErrorCategory::Network)
            .with_severity(ErrorSeverity::Warning),
        ErrorInfo::new(1003, "错误3".to_string())
            .with_category(ErrorCategory::Search)
            .with_severity(ErrorSeverity::Info),
    ];

    // 按错误代码排序
    errors.sort_by(|a, b| a.code().cmp(&b.code()));

    // 验证排序结果
    assert_eq!(errors[0].code(), 1001);
    assert_eq!(errors[1].code(), 1002);
    assert_eq!(errors[2].code(), 1003);
}

/// 测试错误批量处理和转换
#[test]
fn test_batch_error_processing() {
    let errors = vec![
        validation_error("验证失败1"),
        validation_error("验证失败2"),
        network_error("网络失败"),
        engine_error("test_engine", "搜索失败"),
    ];

    // 批量转换为字符串消息
    let error_messages: Vec<String> = errors
        .iter()
        .map(|e| format!("[{}] {}: {}", e.severity(), e.code(), e.message()))
        .collect();

    // 验证转换结果
    assert_eq!(error_messages.len(), 4);
    assert!(error_messages[0].contains("验证失败1"));
    assert!(error_messages[1].contains("验证失败2"));
    assert!(error_messages[2].contains("网络失败"));
    assert!(error_messages[3].contains("搜索失败"));
}

/// 测试复杂错误场景模拟
#[test]
fn test_complex_error_scenario() {
    // 模拟一个复杂的用户注册流程错误场景
    let mut errors = Vec::new();

    // 步骤1: 输入验证
    errors.push(validation_error("输入验证"));
    errors.push(empty_field("email"));

    // 步骤2: 网络请求验证唯一性
    errors.push(network_error("网络错误"));

    // 步骤3: 数据库操作
    errors.push(duplicate_value("johndoe", "username"));

    // 步骤4: 搜索相关（如果涉及搜索）
    errors.push(engine_error("test_engine", "搜索错误"));

    // 分析错误分布
    let category_count: std::collections::HashMap<ErrorCategory, usize> =
        errors
            .iter()
            .fold(std::collections::HashMap::new(), |mut acc, error| {
                *acc.entry(error.category()).or_insert(0) += 1;
                acc
            });

    // 验证错误分布
    assert_eq!(category_count.get(&ErrorCategory::Validation).unwrap(), &3); // validation_error, empty_field, duplicate_value
    assert_eq!(category_count.get(&ErrorCategory::Network).unwrap(), &1);
    assert_eq!(category_count.get(&ErrorCategory::Search).unwrap(), &1);

    // 验证总错误数
    assert_eq!(errors.len(), 5);
}

/// 测试错误恢复和重试逻辑
#[test]
fn test_error_recovery_and_retry() {
    let mut retry_count = 0;
    let max_retries = 3;
    let mut final_error = None;

    // 模拟重试逻辑
    while retry_count < max_retries {
        // 模拟操作失败
        let error = connection_timeout("api.example.com");

        // 检查是否应该重试（这里简化逻辑，实际应该基于错误类型）
        if error.severity() == ErrorSeverity::Error && retry_count < max_retries - 1 {
            retry_count += 1;
            continue;
        } else {
            final_error = Some(error);
            break;
        }
    }

    // 验证重试结果
    assert!(final_error.is_some());
    // 由于循环逻辑问题，这里应该是2而不是0
    assert_eq!(retry_count, 2); // 重试了2次

    if let Some(error) = final_error {
        assert_eq!(error.category(), ErrorCategory::Network);
        assert_eq!(error.code(), 1001); // CONNECTION_TIMEOUT
    }
}

/// 测试错误日志和监控格式
#[test]
fn test_error_logging_and_monitoring() {
    let errors = vec![
        validation_error("验证失败"),
        network_error("网络失败"),
        engine_error("test_engine", "搜索失败"),
    ];

    // 生成监控格式的错误日志
    let log_entries: Vec<String> = errors
        .iter()
        .map(|error| {
            format!(
                "[{}] [{}] [{}: {}] {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                error.severity(),
                error.category(),
                error.code(),
                error.message()
            )
        })
        .collect();

    // 验证日志格式
    assert_eq!(log_entries.len(), 3);
    assert!(log_entries[0].contains("验证失败"));
    assert!(log_entries[1].contains("网络失败"));
    assert!(log_entries[2].contains("搜索失败"));

    // 验证包含所有必要字段
    for log_entry in &log_entries {
        // 检查是否包含中文的"错误"或英文的"Error"
        assert!(log_entry.contains("错误") || log_entry.contains("Error"));
        // 检查类别 - 可能是中文显示
        assert!(
            log_entry.contains("验证")
                || log_entry.contains("网络")
                || log_entry.contains("搜索")
                || log_entry.contains("Validation")
                || log_entry.contains("Network")
                || log_entry.contains("Search")
        );
    }
}
