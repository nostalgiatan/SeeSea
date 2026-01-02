# 配置模块文档

## 概述

配置模块是 SeeSea 的核心组件之一，负责处理所有配置相关的功能。该模块提供了完整的配置管理解决方案，支持多环境配置、动态加载、配置验证和类型安全的配置访问。

## 核心类

### PyConfig

基础配置类，提供简单的配置管理功能。

#### 构造函数

```python
config = PyConfig()
```

#### 属性

##### debug
调试模式开关。

**类型：** bool
**默认值：** False

##### max_results
最大结果数量限制。

**类型：** int
**默认值：** 100

##### timeout_seconds
超时时间（秒）。

**类型：** int
**默认值：** 30

## 配置结构

### SeeSeaConfig

主配置结构，包含所有模块的配置信息。

#### 配置组件

##### general
通用配置，包含环境信息、调试模式等基础设置。

**包含字段：**
- `environment`: 环境标识（development/testing/production）
- `debug`: 调试模式开关
- `log_level`: 日志级别
- `max_workers`: 最大工作线程数

##### server
服务器配置，控制 HTTP 服务器行为。

**包含字段：**
- `host`: 服务器监听地址
- `port`: 服务器监听端口
- `secret_key`: 应用密钥
- `max_connections`: 最大连接数
- `keep_alive_timeout`: 连接保持超时时间

##### search
搜索配置，控制搜索引擎行为。

**包含字段：**
- `max_concurrent_engines`: 最大并发搜索引擎数
- `search_timeout`: 搜索超时时间（秒）
- `formats`: 支持的返回格式列表
- `safe_search_level`: 安全搜索级别
- `include_deepweb`: 是否包含深网搜索

##### privacy
隐私保护配置，控制用户隐私相关功能。

**包含字段：**
- `fingerprint_level`: 浏览器指纹保护级别
- `user_agent_rotation`: 用户代理轮换策略
- `proxy_enabled`: 是否启用代理
- `tor_enabled`: 是否启用 Tor 网络

##### cache
缓存配置，控制各种缓存行为。

**包含字段：**
- `cache_size_mb`: 缓存大小（MB）
- `cache_ttl_seconds`: 缓存过期时间（秒）
- `semantic_cache_enabled`: 是否启用语义缓存
- `bloom_filter_enabled`: 是否启用布隆过滤器

##### api
API 配置，控制 API 接口行为。

**包含字段：**
- `rate_limit_enabled`: 是否启用速率限制
- `rate_limit_requests`: 速率限制请求数
- `rate_limit_window`: 速率限制时间窗口（秒）
- `cors_enabled`: 是否启用 CORS
- `auth_required`: 是否需要认证

##### logging
日志配置，控制日志记录行为。

**包含字段：**
- `level`: 日志级别（trace/debug/info/warn/error）
- `format`: 日志格式
- `file_enabled`: 是否写入文件
- `file_path`: 日志文件路径
- `max_file_size_mb`: 日志文件最大大小（MB）

##### engines
搜索引擎配置，包含各搜索引擎的专用配置。

**包含字段：**
- `bing`: Bing 搜索引擎配置
- `baidu`: 百度搜索引擎配置
- `sogou`: 搜狗搜索引擎配置
- `google`: Google 搜索引擎配置
- 其他搜索引擎配置...

##### vector_store
向量数据库配置，控制向量存储和检索。

**包含字段：**
- `provider`: 向量数据库提供商
- `host`: 向量数据库地址
- `port`: 向量数据库端口
- `collection_name`: 集合名称
- `embedding_model`: 嵌入模型配置

##### network
网络配置，控制网络连接行为。

**包含字段：**
- `dns_over_https`: 是否启用 DoH
- `connection_timeout`: 连接超时时间（秒）
- `request_timeout`: 请求超时时间（秒）
- `max_redirects`: 最大重定向次数
- `user_agent`: 用户代理字符串

## 配置函数

### init_config()

初始化全局配置。

**参数：**
- `environment` (str): 环境名称（development/testing/production）

**返回值：**
无

**异常：**
- `PyRuntimeError`: 配置初始化失败

**示例：**
```python
try:
    init_config("development")
    print("配置初始化成功")
except RuntimeError as e:
    print(f"配置初始化失败: {e}")
```

## 环境配置

### 开发环境配置

```python
# 开发环境默认配置
development_config = {
    "environment": "development",
    "debug": True,
    "logging": {
        "level": "debug",
        "file_enabled": True
    },
    "search": {
        "max_concurrent_engines": 5,
        "search_timeout": 30
    },
    "server": {
        "host": "127.0.0.1",
        "port": 8080,
        "secret_key": "development-secret-key"
    }
}
```

### 测试环境配置

```python
# 测试环境默认配置
testing_config = {
    "environment": "testing",
    "debug": True,
    "logging": {
        "level": "info",
        "file_enabled": True
    },
    "search": {
        "max_concurrent_engines": 8,
        "search_timeout": 45
    }
}
```

### 生产环境配置

```python
# 生产环境默认配置
production_config = {
    "environment": "production",
    "debug": False,
    "logging": {
        "level": "warn",
        "file_enabled": True,
        "max_file_size_mb": 100
    },
    "search": {
        "max_concurrent_engines": 10,
        "search_timeout": 60
    },
    "cache": {
        "cache_size_mb": 512,
        "cache_ttl_seconds": 3600
    }
}
```

## 配置加载

### 配置文件格式

支持多种配置文件格式：
- JSON (.json)
- TOML (.toml)
- YAML (.yaml/.yml)

### 配置加载顺序

1. 默认配置
2. 配置文件
3. 环境变量
4. 命令行参数

### 环境变量映射

```bash
# 通用配置
SEESEA_ENVIRONMENT=development
SEESEA_DEBUG=true
SEESEA_LOG_LEVEL=debug

# 服务器配置
SEESEA_SERVER_HOST=0.0.0.0
SEESEA_SERVER_PORT=8080
SEESEA_SERVER_SECRET_KEY=your-secret-key

# 搜索配置
SEESEA_SEARCH_MAX_CONCURRENT_ENGINES=5
SEESEA_SEARCH_TIMEOUT=30
SEESEA_SEARCH_SAFE_SEARCH_LEVEL=moderate

# 缓存配置
SEESEA_CACHE_SIZE_MB=256
SEESEA_CACHE_TTL_SECONDS=1800

# 网络配置
SEESEA_NETWORK_CONNECTION_TIMEOUT=10
SEESEA_NETWORK_REQUEST_TIMEOUT=30
SEESEA_NETWORK_DNS_OVER_HTTPS=true
```

## 配置验证

### 验证规则

- 端口范围：1-65535
- 超时时间：> 0
- 并发数：> 0
- 密钥长度：≥ 16 字符
- 文件路径：必须可写

### 验证示例

```python
# 验证配置有效性
def validate_config(config_dict):
    """验证配置字典的有效性"""
    required_fields = ["environment", "server", "search"]
    
    for field in required_fields:
        if field not in config_dict:
            raise ValueError(f"缺少必需字段: {field}")
    
    # 验证服务器配置
    server = config_dict["server"]
    if not (1 <= server.get("port", 0) <= 65535):
        raise ValueError("服务器端口必须在 1-65535 范围内")
    
    # 验证搜索配置
    search = config_dict["search"]
    if search.get("max_concurrent_engines", 0) <= 0:
        raise ValueError("并发搜索引擎数必须大于 0")
    
    return True
```

## 动态配置更新

### 运行时配置更新

```python
# 获取当前配置快照
current_config = get_current_config()

# 修改配置
updated_config = current_config.copy()
updated_config["search"]["max_concurrent_engines"] = 15

# 应用新配置
try:
    apply_config(updated_config)
    print("配置更新成功")
except Exception as e:
    print(f"配置更新失败: {e}")
```

### 配置热重载

```python
# 监听配置文件变化
def watch_config_file(filepath):
    """监听配置文件变化并自动重载"""
    import time
    import os
    
    last_modified = os.path.getmtime(filepath)
    
    while True:
        current_modified = os.path.getmtime(filepath)
        if current_modified != last_modified:
            try:
                init_config("production")  # 重新加载配置
                print("配置已自动重载")
                last_modified = current_modified
            except Exception as e:
                print(f"配置重载失败: {e}")
        
        time.sleep(5)  # 每5秒检查一次
```

## 配置最佳实践

### 1. 环境分离
- 严格分离不同环境的配置
- 避免在生产环境启用调试模式
- 使用不同的密钥和凭据

### 2. 安全配置
- 不在代码中硬编码敏感信息
- 使用环境变量存储密钥
- 定期轮换敏感配置

### 3. 性能调优
- 根据实际负载调整并发参数
- 合理设置缓存大小和过期时间
- 监控配置变更对性能的影响

### 4. 监控和告警
- 监控配置加载失败
- 设置配置异常告警
- 记录配置变更历史

## 错误处理

配置模块可能抛出以下异常：

- `PyRuntimeError`: 配置初始化失败、文件读取错误
- `PyValueError`: 配置格式错误、参数验证失败
- `PyFileNotFoundError`: 配置文件不存在

**建议的错误处理：**
```python
try:
    init_config("production")
except RuntimeError as e:
    # 降级到默认配置
    init_config("development")
    print(f"使用开发配置: {e}")
except ValueError as e:
    print(f"配置格式错误: {e}")
    # 使用硬编码的默认配置
except FileNotFoundError as e:
    print(f"配置文件不存在: {e}")
    # 创建默认配置文件
```

## 相关模块

- [系统控制模块](system_controller.md): 系统资源管理和监控
- [日志模块](logging.md): 配置日志记录行为
- [网络模块](network.md): 网络连接配置
- [缓存模块](cache.md): 缓存策略配置

## 注意事项

1. **配置一致性**: 确保不同环境的配置保持一致性
2. **向后兼容**: 配置变更时考虑向后兼容性
3. **配置文档**: 及时更新配置文档和示例
4. **版本控制**: 配置文件纳入版本控制系统
5. **备份策略**: 定期备份重要配置文件
6. **权限管理**: 配置文件设置适当的访问权限