# FAQ (常见问题解答)

## 1. 搜索结果批处理为零的问题

### 问题描述
在Linux环境下使用 `seesea search -p 爱莉希雅` 命令时，出现以下输出：
```
llama_context: n_ctx_per_seq (1024) < n_ctx_train (32768) -- the full capacity of the model will not be utilized
init: embeddings required but some input tokens were not marked as outputs -> overriding
批处理结果：总共处理了 0 个新文档或更新了 0 个现有文档
```

### 根本原因
当只安装了 `playwright` 但没有安装 `chromium` 浏览器时，会导致以下问题：
1. WebCrawler 无法正常工作，无法获取SPA类型链接的渲染HTML
2. 所有URL被标记为需要SPA处理，但实际无法获取内容
3. 内容处理失败，导致没有文档被添加到批处理队列
4. 最终批处理结果为零文档

### 解决方案
安装 `chromium` 浏览器：

```bash
# 使用 playwright 命令安装 chromium
playwright install chromium

# 或者使用系统包管理器安装
# Ubuntu/Debian
sudo apt-get install chromium-browser

# CentOS/RHEL
sudo dnf install chromium

# macOS
brew install --cask chromium
```

### 验证方法
运行搜索命令，检查是否能正常处理文档：
```bash
seesea search -p 爱莉希雅
```
如果输出中显示处理了非零数量的文档，则问题已解决。

## 2. 其他常见问题

### 2.1 嵌入模型加载失败

#### 问题描述
```
Failed to initialize Qwen3 embedding model with llama-cpp-python
```

#### 解决方案
1. 确保模型文件完整
2. 检查模型路径是否正确
3. 确保 llama-cpp-python 已正确安装
4. 尝试重新下载模型

### 2.2 HTTP 请求失败

#### 问题描述
```
Failed to write file: error decoding response body
```

#### 解决方案
1. 检查网络连接
2. 检查代理设置
3. 确保目标网站可访问
4. 尝试使用不同的网络环境

### 2.3 向量数据库连接失败

#### 问题描述
```
Failed to open vector database
```

#### 解决方案
1. 检查数据库文件权限
2. 确保磁盘空间充足
3. 检查数据库路径是否正确
4. 尝试重新初始化数据库

## 3. 性能优化建议

### 3.1 提高搜索速度
1. 减少返回结果数量（使用 `-k` 参数）
2. 使用更高效的嵌入模型
3. 增加线程数（使用 `n_threads` 参数）
4. 确保系统内存充足

### 3.2 降低内存使用
1. 减少批处理大小（使用 `batch_size` 参数）
2. 关闭不必要的功能
3. 使用较小的嵌入模型
4. 定期清理向量数据库

## 4. 跨平台兼容性

### 4.1 Windows 特有问题
- 确保使用管理员权限运行命令
- 检查防火墙设置
- 确保 Visual C++ 运行时已安装

### 4.2 Linux 特有问题
- 确保依赖包已安装
- 检查 SELinux 或 AppArmor 设置
- 确保文件权限正确

### 4.3 macOS 特有问题
- 确保使用 Homebrew 安装的依赖路径正确
- 检查系统安全设置
- 确保 Python 环境正确

## 5. 高级配置

### 5.1 自定义模型路径
```bash
seesea search -p 爱莉希雅 --model-path /path/to/model.gguf
```

### 5.2 调整线程数
```bash
seesea search -p 爱莉希雅 --n-threads 8
```

### 5.3 自定义存储路径
```bash
seesea search -p 爱莉希雅 --store-path /path/to/store.db
```

## 6. 日志和调试

### 6.1 查看详细日志
```bash
seesea search -p 爱莉希雅 --verbose
```

### 6.2 检查配置文件
```bash
cat ~/.config/seesea/config.toml
```

### 6.3 测试嵌入模型
```bash
python -c "from tf.embeddings import TextEmbedder; embedder = TextEmbedder(); print(embedder.encode('test'))"
```

## 7. 社区支持

如果遇到其他问题，可以通过以下方式获取支持：

- 查看项目 GitHub Issues
- 加入社区讨论
- 提交 Bug 报告
- 查阅官方文档

## 8. 更新和升级

### 8.1 升级 SeeSea
```bash
pip install --upgrade seesea
```

### 8.2 更新嵌入模型
```bash
# 删除旧模型
rm -rf ~/.tf/models
# 重新运行搜索命令，会自动下载最新模型
seesea search -p 爱莉希雅
```

### 8.3 更新依赖包
```bash
pip install --upgrade -r requirements.txt
```

## 9. 安全注意事项

### 9.1 隐私保护
- 避免搜索敏感信息
- 定期清理搜索历史
- 检查代理设置
- 确保 HTTPS 连接

### 9.2 安全更新
- 定期更新 SeeSea 和依赖包
- 关注安全公告
- 及时修复漏洞

## 10. 卸载和清理

### 10.1 卸载 SeeSea
```bash
pip uninstall seesea
```

### 10.2 清理数据
```bash
# 删除向量数据库
rm -rf ~/.tf
# 删除配置文件
rm -rf ~/.config/seesea
```

---

如果您遇到其他未在本FAQ中列出的问题，请查看官方文档或提交Issue获取帮助。