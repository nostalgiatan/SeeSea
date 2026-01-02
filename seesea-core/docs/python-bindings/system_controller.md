# 系统控制器模块

系统控制器模块提供动态资源管理和组件并发控制功能，确保系统在高负载下稳定运行。

## 核心功能

系统控制器负责：
- **资源监控**: 实时监控系统资源使用情况
- **并发控制**: 动态调整组件并发数量
- **优先级管理**: 管理组件执行优先级
- **负载均衡**: 在多个组件间分配系统资源
- **故障恢复**: 自动处理组件故障和恢复

## 初始化函数

### init_system_controller()

初始化全局系统控制器。

**函数签名:**
```python
init_system_controller() -> None
```

**说明:**
系统控制器采用单例模式，全局只有一个实例。该函数确保系统控制器已初始化并准备就绪。

**示例:**
```python
# 初始化系统控制器
init_system_controller()
print("系统控制器已初始化")
```

## 组件管理函数

### register_component()

注册组件到系统控制器。

**函数签名:**
```python
register_component(
    component_type: str,
    component_name: str,
    priority: int,
    max_resource_usage: float,
    min_resource_allocation: float
) -> None
```

**参数：**
- `component_type` (str): 组件类型，支持以下值：
  - `"vector_store"`: 向量存储组件
  - `"pro_processor"`: Pro处理器组件
  - `"crawl4ai"`: Crawl4AI组件
- `component_name` (str): 组件实例名称
- `priority` (int): 优先级，范围0-100，数值越大优先级越高
- `max_resource_usage` (float): 最大资源使用率，范围0.0-1.0
- `min_resource_allocation` (float): 最小资源分配，范围0.0-1.0

**示例:**
```python
# 注册向量存储组件
register_component(
    component_type="vector_store",
    component_name="main_vector_store",
    priority=80,
    max_resource_usage=0.7,
    min_resource_allocation=0.1
)

# 注册Crawl4AI组件
register_component(
    component_type="crawl4ai",
    component_name="web_crawler",
    priority=60,
    max_resource_usage=0.5,
    min_resource_allocation=0.05
)
```

## 并发调整函数

### adjust_component_concurrency()

调整组件的并发数量。

**函数签名:**
```python
adjust_component_concurrency(
    component_type: str,
    component_name: str,
    concurrency: int
) -> None
```

**参数：**
- `component_type` (str): 组件类型
- `component_name` (str): 组件实例名称
- `concurrency` (int): 新的并发数量

**示例:**
```python
# 调整向量存储并发数
adjust_component_concurrency(
    component_type="vector_store",
    component_name="main_vector_store",
    concurrency=8
)

# 调整Crawl4AI并发数
adjust_component_concurrency(
    component_type="crawl4ai",
    component_name="web_crawler",
    concurrency=5
)
```

### adjust_component_priority()

调整组件的优先级。

**函数签名:**
```python
adjust_component_priority(
    component_type: str,
    component_name: str,
    priority: int
) -> None
```

**参数：**
- `component_type` (str): 组件类型
- `component_name` (str): 组件实例名称
- `priority` (int): 新的优先级，范围0-100

**示例:**
```python
# 提高向量存储优先级
adjust_component_priority(
    component_type="vector_store",
    component_name="main_vector_store",
    priority=90
)

# 降低Crawl4AI优先级
adjust_component_priority(
    component_type="crawl4ai",
    component_name="web_crawler",
    priority=40
)
```

## 快捷调整函数

### adjust_crawl4ai_concurrency()

专门用于调整Crawl4AI的并发数量。

**函数签名:**
```python
adjust_crawl4ai_concurrency(concurrency: int) -> None
```

**示例:**
```python
# 调整Crawl4AI并发数为10
adjust_crawl4ai_concurrency(10)
```

### adjust_pro_processor_concurrency()

专门用于调整Pro处理器的并发数量。

**函数签名:**
```python
adjust_pro_processor_concurrency(concurrency: int) -> None
```

**示例:**
```python
# 调整Pro处理器并发数为6
adjust_pro_processor_concurrency(6)
```

## 状态查询函数

### get_system_status()

获取系统状态信息。

**函数签名:**
```python
get_system_status() -> dict
```

**返回值：**
- `dict`: 系统状态信息，包含以下字段：
  - `resource_status` (dict): 资源状态
    - `cpu_usage` (float): CPU使用率（0.0-1.0）
    - `memory_usage` (float): 内存使用率（0.0-1.0）
    - `disk_io_usage` (float): 磁盘I/O使用率（0.0-1.0）
    - `network_io_usage` (float): 网络I/O使用率（0.0-1.0）
    - `available_memory` (int): 可用内存（字节）
    - `available_disk` (int): 可用磁盘空间（字节）
    - `total_disk` (int): 磁盘总空间（字节）
    - `disk_usage_percent` (float): 磁盘使用率（0.0-1.0）
    - `load_avg_1` (float): 1分钟系统负载
    - `load_avg_5` (float): 5分钟系统负载
    - `load_avg_15` (float): 15分钟系统负载
  - `running` (bool): 系统控制器是否正在运行
  - `should_terminate` (bool): 系统是否应该终止

**示例:**
```python
status = get_system_status()

print("系统资源状态:")
print(f"CPU使用率: {status['resource_status']['cpu_usage']:.2%}")
print(f"内存使用率: {status['resource_status']['memory_usage']:.2%}")
print(f"磁盘使用率: {status['resource_status']['disk_usage_percent']:.2%}")
print(f"系统负载: {status['resource_status']['load_avg_1']:.2f}")

print(f"\n系统运行状态: {'运行中' if status['running'] else '已停止'}")
print(f"终止标志: {'是' if status['should_terminate'] else '否'}")
```

### should_terminate()

检查系统是否应该终止。

**函数签名:**
```python
should_terminate() -> bool
```

**返回值：**
- `bool`: 如果系统应该终止返回True，否则返回False

**示例:**
```python
if should_terminate():
    print("系统收到终止信号，准备关闭")
    # 执行清理操作
    cleanup()
else:
    print("系统正常运行")
```

## 守护进程函数

### start_system_controller_daemon()

启动系统控制器守护进程。

**函数签名:**
```python
start_system_controller_daemon() -> None
```

**说明:**
在后台启动系统控制器守护进程，持续监控系统资源和动态调整组件行为。

**示例:**
```python
# 启动守护进程
start_system_controller_daemon()
print("系统控制器守护进程已启动")
```

### stop_system_controller_daemon()

停止系统控制器守护进程。

**函数签名:**
```python
stop_system_controller_daemon() -> None
```

**示例:**
```python
# 停止守护进程
stop_system_controller_daemon()
print("系统控制器守护进程已停止")
```

## 资源配置函数

### set_resource_threshold()

设置系统资源阈值。

**函数签名:**
```python
set_resource_threshold(threshold: float) -> None
```

**参数：**
- `threshold` (float): 资源阈值，范围0.0-1.0

**说明:**
当前版本不支持动态修改配置，该函数返回成功但不做实际操作。后续版本将支持动态配置。

## 使用场景

### 动态负载管理
```python
# 初始化系统控制器
init_system_controller()

# 注册组件
register_component(
    component_type="vector_store",
    component_name="main_store",
    priority=80,
    max_resource_usage=0.7,
    min_resource_allocation=0.1
)

# 启动守护进程
start_system_controller_daemon()

# 监控系统状态
def monitor_system():
    while True:
        status = get_system_status()
        
        # 如果CPU使用率过高，降低并发
        if status['resource_status']['cpu_usage'] > 0.8:
            adjust_crawl4ai_concurrency(3)
            adjust_pro_processor_concurrency(2)
        
        # 如果系统负载较低，提高并发
        elif status['resource_status']['load_avg_1'] < 0.5:
            adjust_crawl4ai_concurrency(10)
            adjust_pro_processor_concurrency(8)
        
        time.sleep(30)  # 每30秒检查一次
```

### 组件生命周期管理
```python
def manage_component_lifecycle():
    # 初始化
    init_system_controller()
    
    # 注册组件
    register_component(
        component_type="crawl4ai",
        component_name="crawler_1",
        priority=70,
        max_resource_usage=0.6,
        min_resource_allocation=0.05
    )
    
    # 启动守护进程
    start_system_controller_daemon()
    
    try:
        # 主循环
        while not should_terminate():
            # 获取系统状态
            status = get_system_status()
            
            # 根据资源使用情况调整
            cpu_usage = status['resource_status']['cpu_usage']
            memory_usage = status['resource_status']['memory_usage']
            
            if cpu_usage > 0.9 or memory_usage > 0.85:
                # 资源紧张，降低并发
                adjust_crawl4ai_concurrency(2)
            elif cpu_usage < 0.3 and memory_usage < 0.5:
                # 资源充足，提高并发
                adjust_crawl4ai_concurrency(8)
            
            time.sleep(10)
    
    finally:
        # 停止守护进程
        stop_system_controller_daemon()
```

## 最佳实践

1. **初始化顺序**: 先初始化系统控制器，再注册组件，最后启动守护进程
2. **资源监控**: 定期检查系统状态，避免资源耗尽
3. **渐进调整**: 并发调整应该渐进式，避免剧烈变化
4. **错误处理**: 妥善处理调整失败的情况
5. **优雅关闭**: 使用终止信号确保系统优雅关闭

## 相关模块

- [配置模块](config.md): 系统控制器相关配置
- [缓存模块](cache.md): 系统状态缓存
- [网络模块](network.md): 网络资源监控