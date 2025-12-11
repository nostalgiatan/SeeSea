# Build Script and CI/CD Fixes - 2025-12-11

## 修改摘要

### 1. build.py - 使用文本替换代替 AST 方法

**原因**: AST 方法复杂且可能有格式问题，文本替换更简单可靠

**修改内容**:
- 移除了 `ast` 模块的导入
- 将 `inject_data_to_template()` 函数从 AST 操作改为简单的文本替换
- 使用 `repr()` 函数生成 Python 字面量字符串
- 使用字符串 `replace()` 方法替换模板中的占位符

**优势**:
- 代码更简单，易于理解和维护
- 不依赖 Python 版本特定的 AST API
- 避免了 AST unparse 可能产生的格式问题
- 生成的代码保持原模板的格式

### 2. GitHub Actions CI/CD 工作流修复

#### 问题 1: `overwrite` 参数错误
**修复**: 第 341 行，将 `overwrite: true` 改为 `overwrite_files: true`

```yaml
# 修改前
overwrite: true

# 修改后
overwrite_files: true
```

#### 问题 2: 缺少目标架构参数
**修复**: 在 maturin build 命令中添加 `--target` 参数

```yaml
# 修改前
maturin build --release --strip

# 修改后
maturin build --release --strip --target ${{ matrix.maturin-target }}
```

这确保了每个构建任务都为正确的目标架构编译。

#### 问题 3: 缺少 ARM 交叉编译工具
**新增**: 为 Linux ARM 构建添加交叉编译工具安装步骤

```yaml
- name: Install cross-compilation tools (Linux ARM only)
  if: matrix.platform == 'linux' && (matrix.arch == 'arm64' || matrix.arch == 'armv7')
  run: |
    sudo apt-get update
    if [ "${{ matrix.arch }}" = "arm64" ]; then
      sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
    elif [ "${{ matrix.arch }}" = "armv7" ]; then
      sudo apt-get install -y gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf
    fi
```

### 3. 新增 Cargo 交叉编译配置

**文件**: `.cargo/config.toml`

**内容**:
```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"

[target.x86_64-unknown-linux-gnu]
linker = "gcc"
```

这个配置文件告诉 Cargo 为不同的目标平台使用正确的链接器。

## 验证

所有修改已通过语法检查：
- ✅ `build.py` - 无错误
- ✅ `.github/workflows/ci-cd.yml` - 无错误
- ✅ `.cargo/config.toml` - 无错误

## 预期结果

修复后，GitHub Actions 工作流应该能够：
1. 正确构建所有平台和架构的包（不再只有 arm64）
2. 成功创建 Release 并上传所有产物
3. 正确处理文件覆盖（使用 `overwrite_files: true`）

## 构建矩阵覆盖

现在支持以下平台和架构组合：

**Linux**:
- x64 (amd64) - Python 3.10, 3.11, 3.12, 3.13, 3.14
- arm64 (aarch64) - Python 3.10, 3.11, 3.12, 3.13, 3.14
- armv7 - Python 3.10, 3.11, 3.12, 3.13, 3.14

**Windows**:
- x64 (amd64) - Python 3.10, 3.11, 3.12, 3.13, 3.14
- arm64 (aarch64) - Python 3.10, 3.11, 3.12, 3.13, 3.14

**macOS**:
- x64 (Intel) - Python 3.10, 3.11, 3.12, 3.13, 3.14
- arm64 (Apple Silicon) - Python 3.10, 3.11, 3.12, 3.13, 3.14

**总计**: 40 个不同的构建配置
