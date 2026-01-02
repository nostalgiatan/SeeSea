# 网络客户端模块

网络客户端模块提供HTTP请求功能，支持GET和POST请求，自动处理运行时和连接管理。

## 核心类

### PyNetClient

网络客户端类，提供统一的HTTP请求接口。

#### 构造函数

```python
client = PyNetClient()
```

创建网络客户端实例。自动检测当前是否存在Tokio运行时：
- 如果已存在运行时，复用现有运行时
- 如果不存在运行时，创建新的运行时

注意：该类通常在内部使用，Python代码中不直接实例化，而是通过其他客户端类自动创建。

#### 方法

##### get()

发送GET请求。

**参数：**
- `url` (str): 请求URL
- `headers` (dict, 可选): 请求头字典

**返回值：**
包含以下字段的字典：
- `status` (int): HTTP状态码
- `headers` (dict): 响应头
- `body` (bytes): 响应内容

**示例：**
```python
response = client.get(
    url="https://api.example.com/data",
    headers={"User-Agent": "SeeSea/1.0"}
)
print(f"状态码: {response['status']}")
print(f"响应内容: {response['body'].decode('utf-8')}")
```

##### post()

发送POST请求。

**参数：**
- `url` (str): 请求URL
- `data` (bytes | str): 请求体数据
- `headers` (dict, 可选): 请求头字典

**返回值：**
包含以下字段的字典：
- `status` (int): HTTP状态码
- `headers` (dict): 响应头
- `body` (bytes): 响应内容

**示例：**
```python
# 发送JSON数据
json_data = '{"key": "value"}'
response = client.post(
    url="https://api.example.com/submit",
    data=json_data.encode('utf-8'),
    headers={
        "Content-Type": "application/json",
        "User-Agent": "SeeSea/1.0"
    }
)

# 发送表单数据
form_data = "name=John&age=30"
response = client.post(
    url="https://api.example.com/form",
    data=form_data,
    headers={
        "Content-Type": "application/x-www-form-urlencoded"
    }
)
```

## 错误处理

网络客户端会自动处理以下错误情况：

- **连接错误**: 无法连接到目标服务器
- **超时错误**: 请求超时
- **DNS解析错误**: 无法解析域名
- **SSL/TLS错误**: HTTPS连接失败

所有错误都会转换为Python异常，可以通过标准的异常处理机制捕获：

```python
try:
    response = client.get("https://invalid-url")
except Exception as e:
    print(f"请求失败: {e}")
```

## 运行时管理

网络客户端自动管理Tokio运行时：

1. **运行时检测**: 检查当前线程是否已有Tokio运行时
2. **运行时复用**: 如果存在运行时，复用现有运行时
3. **运行时创建**: 如果不存在运行时，创建新的运行时
4. **资源清理**: 自动处理连接池和资源的清理

## 连接池管理

网络客户端内部使用连接池来优化性能：

- **连接复用**: 自动复用HTTP连接
- **连接池大小**: 根据系统资源自动调整
- **连接超时**: 自动处理空闲连接超时
- **DNS缓存**: 缓存DNS解析结果

## 使用场景

网络客户端主要用于以下场景：

1. **API调用**: 调用RESTful API接口
2. **数据获取**: 获取网页内容或数据
3. **表单提交**: 提交表单数据
4. **文件上传**: 上传文件到服务器
5. **代理请求**: 通过代理服务器发送请求

## 性能优化

网络客户端提供了以下性能优化：

- **连接池**: 复用TCP连接，减少连接建立开销
- **DNS缓存**: 缓存DNS解析结果，减少DNS查询时间
- **请求复用**: 复用HTTP/2连接，支持多路复用
- **压缩支持**: 自动处理gzip/deflate压缩

## 安全特性

网络客户端包含以下安全特性：

- **SSL/TLS验证**: 验证服务器证书
- **证书链验证**: 验证证书链完整性
- **主机名验证**: 验证证书主机名匹配
- **协议版本**: 使用安全的TLS协议版本

## 最佳实践

1. **错误处理**: 始终处理可能的网络错误
2. **超时设置**: 为请求设置合理的超时时间
3. **重试机制**: 对失败的请求实施重试策略
4. **连接池**: 复用客户端实例，避免频繁创建
5. **资源清理**: 及时关闭不再使用的客户端

## 相关模块

- [搜索模块](search.md): 使用网络客户端进行搜索请求
- [RSS模块](rss.md): 使用网络客户端获取RSS源
- [缓存模块](cache.md): 缓存网络请求结果