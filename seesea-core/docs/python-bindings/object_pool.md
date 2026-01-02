# 对象池管理模块

对象池管理模块提供DatePage对象的高效复用机制，通过对象池模式减少对象创建和销毁的开销，提高系统性能。该模块支持动态调整池大小，并提供详细的统计信息。

## 核心类

### PyDatePageObjectPool

DatePage对象池类，管理DatePage对象的生命周期和复用。

#### 构造函数

```python
pool = PyDatePageObjectPool(min_size=10, max_size=100)
```

创建DatePage对象池实例。

**参数:**
- `min_size` (int, 可选): 对象池最小大小，默认为10
- `max_size` (int, 可选): 对象池最大大小，默认为100

#### 方法

##### get()

从对象池获取一个DatePage对象。

**函数签名:**
```python
get() -> PyDatePage
```

**返回值:**
- `PyDatePage`: 从池中获取的DatePage对象，如果池为空则创建新对象

**说明:**
该方法实现了对象池的核心功能：
- 如果池中有可用对象，返回池中的对象并重置其状态
- 如果池为空，创建新的DatePage对象
- 自动更新命中率统计信息

**示例:**
```python
from seesea_python_bindings import PyDatePageObjectPool

pool = PyDatePageObjectPool(min_size=5, max_size=50)

# 获取对象
date_page = pool.get()
print(f"获取对象成功: {date_page is not None}")
```

##### put()

将DatePage对象归还到对象池。

**函数签名:**
```python
put(py_date_page: PyDatePage) -> None
```

**参数:**
- `py_date_page` (PyDatePage): 要归还的DatePage对象

**说明:**
该方法实现了对象的回收和复用：
- 重置对象状态，清除所有数据
- 如果池未满，将对象放回池中
- 如果池已满，对象将被丢弃
- 自动更新回收统计信息

**示例:**
```python
# 获取并归还对象
date_page = pool.get()
# 使用对象...
pool.put(date_page)  # 归还对象到池
```

##### set_max_size()

设置对象池的最大大小。

**函数签名:**
```python
set_max_size(max_size: int) -> None
```

**参数:**
- `max_size` (int): 新的最大大小

**说明:**
动态调整对象池的最大容量：
- 如果新大小小于当前大小，池会自动收缩
- 如果新大小大于当前大小，池会保持当前状态直到需要扩展

**示例:**
```python
# 初始配置
pool = PyDatePageObjectPool(max_size=100)

# 根据系统负载调整池大小
pool.set_max_size(200)  # 增加池容量
pool.set_max_size(50)   # 减少池容量
```

##### set_min_size()

设置对象池的最小大小。

**函数签名:**
```python
set_min_size(min_size: int) -> None
```

**参数:**
- `min_size` (int): 新的最小大小

**说明:**
动态调整对象池的最小容量：
- 如果新大小大于当前最小大小，池会预填充到新的最小大小
- 如果新大小小于当前最小大小，池会保持当前对象数量

**示例:**
```python
# 初始配置
pool = PyDatePageObjectPool(min_size=10)

# 根据需求调整最小大小
pool.set_min_size(20)  # 增加最小对象数量
pool.set_min_size(5)   # 减少最小对象数量
```

##### current_size()

获取当前池大小。

**函数签名:**
```python
current_size() -> int
```

**返回值:**
- `int`: 当前池中对象的数量

**示例:**
```python
pool = PyDatePageObjectPool(min_size=10, max_size=100)
print(f"当前池大小: {pool.current_size()}")

# 获取几个对象
date_page1 = pool.get()
date_page2 = pool.get()
print(f"获取对象后池大小: {pool.current_size()}")

# 归还对象
pool.put(date_page1)
print(f"归还对象后池大小: {pool.current_size()}")
```

##### max_size()

获取池的最大大小。

**函数签名:**
```python
max_size() -> int
```

**返回值:**
- `int`: 池的最大容量

##### min_size()

获取池的最小大小。

**函数签名:**
```python
min_size() -> int
```

**返回值:**
- `int`: 池的最小容量

##### hit_rate()

获取池的命中率。

**函数签名:**
```python
hit_rate() -> float
```

**返回值:**
- `float`: 命中率（0.0-1.0），表示从池中成功获取对象的比例

**说明:**
命中率是衡量对象池效率的重要指标：
- 高命中率（接近1.0）表示池化效果好，大部分请求都能从池中获取对象
- 低命中率（接近0.0）表示池化效果差，大部分请求都需要创建新对象

**示例:**
```python
pool = PyDatePageObjectPool(min_size=20, max_size=100)

# 模拟多次获取操作
for i in range(100):
    obj = pool.get()
    if i % 2 == 0:  # 只归还一半的对象
        pool.put(obj)

hit_rate = pool.hit_rate()
print(f"池命中率: {hit_rate:.2%}")
```

##### stats()

获取池的详细统计信息。

**函数签名:**
```python
stats() -> PyDatePageObjectPoolStats
```

**返回值:**
- `PyDatePageObjectPoolStats`: 包含详细统计信息的统计对象

**示例:**
```python
stats = pool.stats()
print(f"当前大小: {stats.current_size}")
print(f"最大大小: {stats.max_size}")
print(f"最小大小: {stats.min_size}")
print(f"命中率: {stats.hits}")
print(f"未命中率: {stats.misses}")
print(f"命中率百分比: {stats.hit_rate:.2%}")
print(f"创建计数: {stats.created}")
print(f"回收计数: {stats.recycled}")
```

##### clear()

清理对象池，移除所有对象。

**函数签名:**
```python
clear() -> None
```

**说明:**
清空对象池中的所有对象，重置池状态：
- 所有池中的对象都会被销毁
- 统计信息不会被重置
- 池会保持配置的最小大小

**示例:**
```python
# 填充池
pool = PyDatePageObjectPool(min_size=30, max_size=100)
print(f"清理前池大小: {pool.current_size()}")

# 清理池
pool.clear()
print(f"清理后池大小: {pool.current_size()}")
```

##### resize()

动态调整池大小到指定大小。

**函数签名:**
```python
resize(desired_size: int) -> None
```

**参数:**
- `desired_size` (int): 期望的池大小

**说明:**
根据当前负载动态调整池大小：
- 如果desired_size大于当前大小，池会扩展到该大小
- 如果desired_size小于当前大小，池会收缩到该大小
- 调整过程会考虑最小和最大大小限制

**示例:**
```python
pool = PyDatePageObjectPool(min_size=10, max_size=100)

# 扩展到指定大小
pool.resize(50)
print(f"扩展后池大小: {pool.current_size()}")

# 收缩到指定大小
pool.resize(20)
print(f"收缩后池大小: {pool.current_size()}")
```

### PyDatePageObjectPoolStats

对象池统计信息类，提供详细的性能和使用统计。

#### 属性

**current_size** (int): 当前池大小

**max_size** (int): 池的最大大小

**min_size** (int): 池的最小大小

**hits** (int): 命中次数（从池中成功获取对象）

**misses** (int): 未命中次数（池为空需要创建新对象）

**hit_rate** (float): 命中率（0.0-1.0）

**created** (int): 创建的对象总数

**recycled** (int): 回收的对象总数

#### 方法

##### to_dict()

将统计信息转换为Python字典。

**函数签名:**
```python
to_dict() -> Dict[str, Union[int, float]]
```

**返回值:**
- `Dict[str, Union[int, float]]`: 包含所有统计信息的字典

**示例:**
```python
stats = pool.stats()
stats_dict = stats.to_dict()
print(f"统计信息字典: {stats_dict}")
```

## 使用示例

### 基本对象池使用

```python
from seesea_python_bindings import PyDatePageObjectPool, PyDatePage

# 创建对象池
pool = PyDatePageObjectPool(min_size=5, max_size=20)

# 获取对象
date_page = pool.get()
print(f"获取对象: {date_page is not None}")

# 使用对象...
# date_page.url = "https://example.com"
# date_page.description = "示例页面"

# 归还对象
pool.put(date_page)
print(f"对象归还成功")

# 检查池状态
print(f"当前池大小: {pool.current_size()}")
print(f"池命中率: {pool.hit_rate():.2%}")
```

### 动态池管理

```python
from seesea_python_bindings import PyDatePageObjectPool

# 创建初始池
pool = PyDatePageObjectPool(min_size=10, max_size=50)

# 监控系统负载变化
import time
import random

def simulate_load():
    """模拟系统负载变化"""
    return random.randint(1, 100)

# 动态调整池大小
for i in range(10):
    load = simulate_load()
    
    if load > 80:
        # 高负载，增加池容量
        pool.set_max_size(100)
        pool.set_min_size(30)
        pool.resize(60)
        print(f"高负载 - 池大小调整到: {pool.current_size()}")
    elif load < 20:
        # 低负载，减少池容量
        pool.set_max_size(30)
        pool.set_min_size(5)
        pool.resize(15)
        print(f"低负载 - 池大小调整到: {pool.current_size()}")
    else:
        # 正常负载，保持中等大小
        pool.resize(25)
        print(f"正常负载 - 池大小: {pool.current_size()}")
    
    time.sleep(0.1)

# 显示最终统计
stats = pool.stats()
print(f"\n最终统计信息:")
print(f"命中率: {stats.hit_rate:.2%}")
print(f"创建对象: {stats.created}")
print(f"回收对象: {stats.recycled}")
```

### 性能监控

```python
from seesea_python_bindings import PyDatePageObjectPool

# 创建池并配置监控
pool = PyDatePageObjectPool(min_size=20, max_size=100)

# 模拟高并发场景
import threading
import time

def worker_thread(thread_id):
    """工作线程，模拟对象使用"""
    for i in range(50):
        # 获取对象
        date_page = pool.get()
        
        # 模拟对象使用
        time.sleep(0.001)  # 模拟处理时间
        
        # 随机决定是否归还（模拟对象泄漏）
        if i % 10 != 0:  # 90%的归还率
            pool.put(date_page)
        
        time.sleep(0.001)

# 启动多个工作线程
threads = []
for i in range(5):
    thread = threading.Thread(target=worker_thread, args=(i,))
    threads.append(thread)
    thread.start()

# 监控线程
def monitor():
    while any(thread.is_alive() for thread in threads):
        stats = pool.stats()
        print(f"监控 - 池大小: {stats.current_size}, "
              f"命中率: {stats.hit_rate:.2%}, "
              f"命中: {stats.hits}, 未命中: {stats.misses}")
        time.sleep(0.5)

monitor_thread = threading.Thread(target=monitor)
monitor_thread.start()

# 等待所有工作线程完成
for thread in threads:
    thread.join()

monitor_thread.join()

# 显示最终统计
final_stats = pool.stats()
print(f"\n最终统计:")
print(f"总命中率: {final_stats.hit_rate:.2%}")
print(f"对象创建: {final_stats.created}")
print(f"对象回收: {final_stats.recycled}")
print(f"池利用率: {final_stats.recycled/final_stats.created*100:.1f}%")
```

### 资源优化

```python
from seesea_python_bindings import PyDatePageObjectPool

# 创建池
pool = PyDatePageObjectPool(min_size=5, max_size=50)

# 分析不同配置的性能
configs = [
    {"min_size": 5, "max_size": 20},
    {"min_size": 10, "max_size": 30},
    {"min_size": 15, "max_size": 40},
    {"min_size": 20, "max_size": 50},
]

for config in configs:
    # 重新配置池
    pool.set_min_size(config["min_size"])
    pool.set_max_size(config["max_size"])
    
    # 模拟使用模式
    import random
    for _ in range(100):
        obj = pool.get()
        # 模拟处理时间变化
        processing_time = random.uniform(0.001, 0.01)
        # 模拟不同的归还模式
        if random.random() > 0.1:  # 90%归还率
            pool.put(obj)
    
    # 获取统计信息
    stats = pool.stats()
    
    print(f"配置 {config}:")
    print(f"  命中率: {stats.hit_rate:.2%}")
    print(f"  池利用率: {stats.recycled/stats.created*100:.1f}%")
    print(f"  当前大小: {stats.current_size}")
    print(f"  效率评分: {stats.hit_rate * stats.recycled/stats.created:.3f}")
    print()

# 找到最优配置
optimal_config = max(configs, key=lambda c: 
    pool.stats().hit_rate * pool.stats().recycled/pool.stats().created)

print(f"最优配置: {optimal_config}")
```

## 错误处理

```python
from seesea_python_bindings import PyDatePageObjectPool

pool = PyDatePageObjectPool()

# 处理异常情况
try:
    # 尝试设置无效的大小
    pool.set_max_size(0)  # 这可能不是最佳实践
    print("设置最大大小为0")
except Exception as e:
    print(f"设置大小时出错: {e}")

# 正确的错误处理
try:
    # 获取对象
    obj = pool.get()
    if obj is None:
        print("无法获取对象")
    else:
        print("成功获取对象")
        # 使用对象...
        pool.put(obj)
except Exception as e:
    print(f"操作对象池时出错: {e}")
```

## 注意事项

1. **对象生命周期**: 从池中获取的对象必须正确归还，否则会导致资源泄漏

2. **线程安全**: 对象池是线程安全的，可以在多线程环境中使用

3. **大小限制**: 设置合理的池大小，避免过度消耗内存或频繁创建销毁对象

4. **监控统计**: 定期监控池的统计信息，了解使用模式和性能状况

5. **动态调整**: 根据系统负载动态调整池大小，优化资源利用率

6. **错误处理**: 妥善处理对象获取和归还过程中的异常情况

7. **性能考虑**: 在高并发场景下，适当增大池大小以提高命中率

8. **内存管理**: 注意池中的对象不会被垃圾回收，确保及时清理不需要的对象