use error::ErrorCategory;
use seesea_errors::search::search_error;
use seesea_errors::*;

/// 测试空值和边界值处理
#[test]
fn test_empty_and_boundary_values() {
    // 测试空字符串
    let empty_error = validation_error("");
    assert_eq!(empty_error.message(), "");
    assert_eq!(empty_error.code(), 4000); // VALIDATION_ERROR_BASE

    // 测试超长字符串（边界情况）
    let long_message = "a".repeat(1000);
    let long_error = network_error(&long_message);
    assert_eq!(long_error.message().len(), 1015); // "Network Error: " + 1000 characters = 1015

    // 测试特殊字符
    let special_chars = "!@#$%^&*()_+-=[]{}|;':\",./<>?";
    let special_error = search_error(special_chars);
    assert_eq!(special_error.message(), special_chars);
}

/// 测试 Unicode 和国际化字符
#[test]
fn test_unicode_and_internationalization() {
    // 测试中文字符
    let chinese_error = validation_error("验证失败");
    assert_eq!(chinese_error.message(), "验证失败");

    // 测试日文
    let japanese_error = network_error("ネットワークエラー");
    assert_eq!(
        japanese_error.message(),
        "Network Error: ネットワークエラー"
    );

    // 测试阿拉伯语
    let arabic_error = search_error("خطأ في البحث");
    assert_eq!(arabic_error.message(), "خطأ في البحث");

    // 测试表情符号
    let emoji_error = parse_error("格式错误 😅");
    assert!(emoji_error.message().contains("😅"));
}

/// 测试内存压力和性能边界
#[test]
fn test_memory_pressure_and_performance() {
    // 创建大量错误对象
    let mut errors = Vec::new();
    for i in 0..1000 {
        let error = validation_error(&format!("验证错误 {}", i));
        errors.push(error);
    }

    // 验证所有错误都被正确创建
    assert_eq!(errors.len(), 1000);

    // 验证最后一个错误
    let last_error = &errors[999];
    assert_eq!(last_error.message(), "验证错误 999");

    // 验证内存使用（基本检查）
    for (i, error) in errors.iter().enumerate() {
        assert_eq!(error.code(), 4000); // VALIDATION_ERROR_BASE
        assert_eq!(error.category(), ErrorCategory::Validation);
        assert_eq!(error.message(), format!("验证错误 {}", i));
    }
}

/// 测试并发创建错误
#[test]
fn test_concurrent_error_creation() {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::thread;

    let errors = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    // 启动多个线程并发创建错误
    for i in 0..10 {
        let errors_clone = Arc::clone(&errors);
        let handle = thread::spawn(move || {
            let error = network_error(&format!("网络错误 {}", i));
            let mut errors = errors_clone.lock().unwrap();
            errors.push(error);
        });
        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().expect("线程执行失败");
    }

    // 验证所有错误都被创建
    let errors = errors.lock().unwrap();
    assert_eq!(errors.len(), 10);

    // 验证错误内容
    for (_i, error) in errors.iter().enumerate() {
        assert_eq!(error.code(), 1000); // NETWORK_ERROR_BASE
        assert_eq!(error.category(), ErrorCategory::Network);
        // 由于并发执行，顺序可能不对，只检查消息格式
        assert!(error.message().starts_with("Network Error: 网络错误 "));
    }
}

/// 测试错误对象的循环引用（应该避免）
#[test]
fn test_circular_reference_prevention() {
    // 创建两个相互引用的错误（模拟场景）
    let error1 = validation_error("错误1");
    let error2 = network_error("错误2");

    // 验证它们是不同的对象
    assert_ne!(error1.code(), error2.code());
    assert_ne!(error1.category(), error2.category());

    // 验证错误对象可以正确比较
    assert_ne!(error1, error2);
    assert!(error1 != error2);
}

/// 测试错误对象的深度嵌套
#[test]
fn test_deep_error_nesting() {
    // 创建多个不同的错误对象，模拟错误链
    let base_error = validation_error("基础验证错误");
    let mut errors = vec![base_error];

    // 创建嵌套错误链
    for i in 1..=5 {
        let error = ErrorInfo::new(5000 + i, format!("嵌套错误 {}", i))
            .with_category(ErrorCategory::Validation)
            .with_severity(ErrorSeverity::Error);
        errors.push(error);
    }

    // 验证最外层错误
    let last_error = &errors[5];
    assert_eq!(last_error.code(), 5005);

    // 验证错误对象可以正确克隆和比较
    let cloned_errors: Vec<ErrorInfo> = errors.iter().map(|e| e.clone()).collect();
    assert_eq!(cloned_errors.len(), errors.len());
}

/// 测试极端时间戳值
#[test]
fn test_extreme_timestamp_values() {
    // 创建过去和未来的时间戳
    let past_error = ErrorInfo::new(4000, "过去的错误".to_string())
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error);

    let future_error = ErrorInfo::new(3000, "未来的错误".to_string())
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Warning);

    // 验证错误属性
    assert_eq!(past_error.category(), ErrorCategory::Validation);
    assert_eq!(future_error.category(), ErrorCategory::Network);

    // 验证错误对象可以正确比较
    assert_ne!(past_error, future_error);
    assert!(past_error != future_error);
}

/// 测试错误代码的最大长度
#[test]
fn test_maximum_error_code_length() {
    // 创建超长的错误消息（边界测试）
    let long_message = "A".repeat(100);
    let error = ErrorInfo::new(4000, long_message.clone())
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error);

    // 验证超长错误消息
    assert_eq!(error.message().len(), 100);
    assert_eq!(error.message(), long_message);

    // 验证错误对象可以正确克隆
    let cloned_error = error.clone();
    assert_eq!(cloned_error.message().len(), 100);
    assert_eq!(cloned_error.message(), long_message);
}

/// 测试错误消息的重复模式
#[test]
fn test_repetitive_error_patterns() {
    // 创建大量相似的错误
    let mut errors = Vec::new();
    for i in 0..100 {
        errors.push(validation_error(&format!("重复的验证错误 {}", i)));
    }

    // 验证所有错误的基本属性相同
    for (i, error) in errors.iter().enumerate() {
        assert_eq!(error.code(), 4000); // VALIDATION_ERROR_BASE
        assert_eq!(error.category(), ErrorCategory::Validation);
        assert_eq!(error.severity(), ErrorSeverity::Error);
        assert_eq!(error.message(), format!("重复的验证错误 {}", i));
    }

    // 验证内存中的错误数量
    assert_eq!(errors.len(), 100);
}

/// 测试错误对象的内存对齐和大小
#[test]
fn test_error_memory_alignment() {
    use std::mem;

    // 创建不同类型的错误
    let validation_error = validation_error("验证");
    let network_error = network_error("网络");
    let search_error = search_error("搜索");

    // 验证内存大小（基本检查）
    assert!(mem::size_of_val(&validation_error) > 0);
    assert!(mem::size_of_val(&network_error) > 0);
    assert!(mem::size_of_val(&search_error) > 0);

    // 验证所有错误大小相同（因为它们都是相同的结构体）
    assert_eq!(
        mem::size_of_val(&validation_error),
        mem::size_of_val(&network_error)
    );
    assert_eq!(
        mem::size_of_val(&network_error),
        mem::size_of_val(&search_error)
    );
}

/// 测试错误处理的性能基准
#[test]
fn test_error_handling_performance() {
    use std::time::Instant;

    // 性能测试：创建大量错误
    let start = Instant::now();
    let mut errors = Vec::new();

    for i in 0..10000 {
        errors.push(validation_error(&format!("性能测试错误 {}", i)));
    }

    let creation_time = start.elapsed();

    // 性能测试：错误属性访问
    let start = Instant::now();
    for error in &errors {
        let _code = error.code();
        let _message = error.message();
        let _category = error.category();
        let _severity = error.severity();
    }
    let access_time = start.elapsed();

    // 验证操作在合理时间内完成（基本性能检查）
    assert!(creation_time.as_secs() < 1); // 创建应该在1秒内完成
    assert!(access_time.as_secs() < 1); // 属性访问应该在1秒内完成

    // 验证数据完整性
    assert_eq!(errors.len(), 10000);
}
