// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! 系统控制器模块
//!
//! 系统调控中心的核心组件，负责协调资源监控、优先级管理和动态调整

use super::config::{ConfigUpdateEvent, ConfigUpdateHandler, DynamicConfigManager};
use super::priority::PriorityManager;
use super::resource::ResourceMonitor;
use super::types::{
    AdjustmentRequest, AdjustmentResponse, ComponentConfig, ComponentId, ComponentStatus,
    SystemControllerConfig, SystemStatus,
};
use once_cell::sync::OnceCell;
use seesea_config::network::NetworkConfig;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, warn};

/// 全局系统控制器实例 - 自动初始化单例
static GLOBAL_SYSTEM_CONTROLLER: OnceCell<Arc<SystemController>> = OnceCell::new();

/// 全局 Tokio 运行时实例 - 供系统控制器使用
static GLOBAL_RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();

/// 全局进程启动时间
static PROCESS_START_TIME: once_cell::sync::Lazy<std::time::Instant> =
    once_cell::sync::Lazy::new(std::time::Instant::now);

/// 获取进程运行时间（秒）
pub fn get_process_uptime_seconds() -> u64 {
    PROCESS_START_TIME.elapsed().as_secs()
}

/// 获取或创建全局 Tokio 运行时
///
/// 确保在没有运行时上下文的情况下也能正常工作
pub fn get_or_create_runtime() -> &'static tokio::runtime::Runtime {
    GLOBAL_RUNTIME.get_or_init(|| {
        info!("创建全局 Tokio 运行时");
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("seesea-global-runtime")
            .build()
            .expect("Failed to create global Tokio runtime")
    })
}

/// 在适当的运行时上下文中执行异步任务
///
/// 如果当前已有 Tokio 运行时，则使用当前运行时
/// 否则使用全局运行时
pub fn spawn_runtime_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // 已在 Tokio 运行时中，直接 spawn
            handle.spawn(future);
        }
        Err(_) => {
            // 不在运行时中，使用全局运行时
            let runtime = get_or_create_runtime();
            runtime.spawn(future);
        }
    }
}

/// 获取全局系统控制器实例，自动初始化
///
/// 此函数可以在任何上下文中安全调用，无论是否存在 Tokio 运行时
pub fn get_global_system_controller() -> Arc<SystemController> {
    GLOBAL_SYSTEM_CONTROLLER
        .get_or_init(|| {
            info!("自动初始化系统调控中心");
            // 使用默认配置创建系统控制器
            let config = SystemControllerConfig::default();
            let controller = Arc::new(SystemController::new(config));

            // 在后台启动系统控制器（使用安全的运行时上下文）
            let controller_clone = controller.clone();
            spawn_runtime_task(async move {
                controller_clone.start().await;
            });

            controller
        })
        .clone()
}

/// 系统控制器
///
/// 系统调控中心的核心组件，负责协调资源监控、优先级管理和动态调整
pub struct SystemController {
    /// 资源监控器
    resource_monitor: Arc<ResourceMonitor>,
    /// 优先级管理器
    priority_manager: Arc<PriorityManager>,
    /// 配置
    config: SystemControllerConfig,
    /// 系统控制器是否运行
    running: Arc<RwLock<bool>>,
    /// 程序终止标志，当磁盘空间不足时设置为true
    should_terminate: Arc<RwLock<bool>>,
    /// 组件状态映射
    component_statuses: Arc<RwLock<std::collections::HashMap<ComponentId, ComponentStatus>>>,
    /// 动态配置管理器
    dynamic_config_manager: Arc<DynamicConfigManager>,
}

impl SystemController {
    /// 创建新的系统控制器
    pub fn new(config: SystemControllerConfig) -> Self {
        let resource_monitor = Arc::new(ResourceMonitor::new(Duration::from_millis(
            config.monitoring_interval_ms,
        )));

        let priority_manager = Arc::new(PriorityManager::new(config.priority_factor));

        // 创建动态配置管理器，使用默认网络配置
        let dynamic_config_manager = Arc::new(DynamicConfigManager::new(NetworkConfig::default()));

        Self {
            resource_monitor,
            priority_manager,
            config,
            running: Arc::new(RwLock::new(false)),
            should_terminate: Arc::new(RwLock::new(false)),
            component_statuses: Arc::new(RwLock::new(std::collections::HashMap::new())),
            dynamic_config_manager,
        }
    }

    /// 检查系统是否应该终止
    pub async fn should_terminate(&self) -> bool {
        *self.should_terminate.read().await
    }

    /// 设置系统终止标志
    pub async fn set_should_terminate(&self, terminate: bool) {
        let mut should_terminate = self.should_terminate.write().await;
        *should_terminate = terminate;
    }

    /// 获取资源监控器
    pub fn resource_monitor(&self) -> Arc<ResourceMonitor> {
        self.resource_monitor.clone()
    }

    /// 获取优先级管理器
    pub fn priority_manager(&self) -> Arc<PriorityManager> {
        self.priority_manager.clone()
    }

    /// 获取动态配置管理器
    pub fn dynamic_config_manager(&self) -> Arc<DynamicConfigManager> {
        self.dynamic_config_manager.clone()
    }

    /// 获取配置
    pub fn config(&self) -> &SystemControllerConfig {
        &self.config
    }

    /// 启动系统控制器
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            debug!("System controller is already running");
            return;
        }
        *running = true;
        drop(running);

        info!("Starting system controller with config: {:?}", self.config);

        // 启动资源监控
        let resource_monitor_clone = self.resource_monitor.clone();
        spawn(async move {
            resource_monitor_clone.start().await;
        });

        // 启动动态调整任务
        let mut adjustment_interval =
            interval(Duration::from_millis(self.config.adjustment_interval_ms));
        debug!(
            "Starting dynamic adjustment task with interval: {:?}",
            self.config.adjustment_interval_ms
        );

        loop {
            adjustment_interval.tick().await;
            match self.perform_dynamic_adjustment().await {
                Ok(_) => {
                    debug!("Dynamic adjustment completed successfully");
                }
                Err(e) => {
                    warn!("Failed to perform dynamic adjustment: {}", e);
                }
            }
        }
    }

    /// 停止系统控制器
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("System controller stopped");
    }

    /// 检查系统控制器是否运行
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// 注册组件
    pub async fn register_component(&self, config: ComponentConfig) -> Result<(), String> {
        // 注册到优先级管理器
        self.priority_manager
            .register_component(config.clone())
            .await?;

        // 初始化组件状态
        let mut statuses = self.component_statuses.write().await;
        let component_status = ComponentStatus {
            component_id: config.id.clone(),
            current_resource_usage: 0.0,
            current_priority: config.priority,
            current_params: config.adjustment_params.clone(),
            healthy: true,
            last_adjustment_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        statuses.insert(config.id.clone(), component_status);

        info!(
            "Component registered: {:?}, priority: {}",
            config.id, config.priority
        );
        Ok(())
    }

    /// 注销组件
    pub async fn unregister_component(&self, component_id: &ComponentId) -> Result<(), String> {
        // 从优先级管理器中注销
        self.priority_manager
            .unregister_component(component_id)
            .await?;

        // 从组件状态映射中移除
        let mut statuses = self.component_statuses.write().await;
        statuses.remove(component_id);

        info!("Component unregistered: {:?}", component_id);
        Ok(())
    }

    /// 获取系统状态
    pub async fn get_system_status(&self) -> SystemStatus {
        let resource_status = self.resource_monitor.get_current_status().await;
        let running = self.is_running().await;
        let statuses = self.component_statuses.read().await;

        SystemStatus {
            resource_status,
            component_statuses: statuses.values().cloned().collect(),
            controller_running: running,
            daemon_running: false, // 需要守护进程实例引用来获取真实状态
        }
    }

    /// 获取组件状态
    pub async fn get_component_status(
        &self,
        component_id: &ComponentId,
    ) -> Option<ComponentStatus> {
        let statuses = self.component_statuses.read().await;
        statuses.get(component_id).cloned()
    }

    /// 更新组件状态
    pub async fn update_component_status(&self, status: ComponentStatus) -> Result<(), String> {
        let mut statuses = self.component_statuses.write().await;
        statuses.insert(status.component_id.clone(), status);
        Ok(())
    }

    /// 处理调整请求
    pub async fn handle_adjustment_request(
        &self,
        request: AdjustmentRequest,
    ) -> AdjustmentResponse {
        // 委托给优先级管理器处理
        let response = self
            .priority_manager
            .handle_adjustment_request(request.clone())
            .await;

        // 如果调整成功，更新组件状态
        if response.success {
            let mut statuses = self.component_statuses.write().await;
            if let Some(status) = statuses.get_mut(&request.component_id) {
                // 根据调整类型更新相应字段
                match request.adjustment_type {
                    super::types::AdjustmentType::AdjustPriority => {
                        // 更新组件状态中的优先级
                        if let Some(priority_value) = response.adjusted_params.get("priority")
                            && let Some(priority) = priority_value.as_u64()
                        {
                            status.current_priority = priority as u8;
                        }
                    }
                    super::types::AdjustmentType::AdjustConcurrency => {
                        // 更新组件状态中的优先级（如果有）
                        if let Some(priority_value) = response.adjusted_params.get("priority")
                            && let Some(priority) = priority_value.as_u64()
                        {
                            status.current_priority = priority as u8;
                        }
                    }
                    _ => {
                        // 其他调整类型，不更新特殊字段
                    }
                }

                // 更新组件状态中的参数
                status.current_params = response.adjusted_params.clone();

                // 更新上次调整时间戳
                status.last_adjustment_timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
        }

        response
    }

    /// 执行动态调整
    async fn perform_dynamic_adjustment(&self) -> Result<(), String> {
        // 获取当前资源状态
        let resource_status = self.resource_monitor.get_current_status().await;

        // 检查磁盘空间，当使用率超过90%时设置终止标志
        if resource_status.disk_usage_percent > 0.9 {
            warn!(
                "磁盘空间不足，使用率: {:.2}%, 设置系统终止标志",
                resource_status.disk_usage_percent * 100.0
            );
            self.set_should_terminate(true).await;
        }

        // 检查资源使用率是否超过阈值
        let need_adjustment = resource_status.cpu_usage > self.config.resource_threshold
            || resource_status.memory_usage > self.config.resource_threshold
            || resource_status.disk_io_usage > self.config.resource_threshold
            || resource_status.network_io_usage > self.config.resource_threshold;

        if !need_adjustment {
            debug!("Resource usage is within threshold, skipping dynamic adjustment");
            return Ok(());
        }

        info!("Resource usage exceeded threshold, performing dynamic adjustment");

        // 计算资源分配
        let resource_allocation = self
            .priority_manager
            .calculate_resource_allocation(&resource_status)
            .await;

        // 为每个组件生成调整建议
        let statuses = self.component_statuses.read().await;
        for (component_id, component_status) in statuses.iter() {
            if let Some(allocation) = resource_allocation.get(component_id) {
                // 计算当前资源使用率与建议分配的差异
                let current_usage = component_status.current_resource_usage;
                let suggested_allocation = *allocation;

                let usage_diff = current_usage - suggested_allocation;

                // 如果差异较大，生成调整请求
                if usage_diff.abs() > 0.1 {
                    // 差异超过10%则进行调整
                    debug!(
                        "Component {:?} has resource usage diff: {:.2}, current: {:.2}, suggested: {:.2}",
                        component_id, usage_diff, current_usage, suggested_allocation
                    );

                    // 这里添加具体的调整逻辑
                    // 根据组件类型执行不同的调整策略
                    match component_id.component_type {
                        super::types::ComponentType::ProProcessor => {
                            // 对于ProProcessor组件，调整DatePage对象池大小
                            // 基于建议分配和当前资源使用率计算新的池大小
                            let current_pool_size = 100; // 这里需要实际获取当前池大小
                            let new_pool_size = match usage_diff.signum() {
                                1.0 => {
                                    // 当前使用率过高，减少池大小
                                    (current_pool_size as f64 * 0.8) as usize
                                }
                                -1.0 => {
                                    // 当前使用率过低，增加池大小
                                    (current_pool_size as f64 * 1.2) as usize
                                }
                                _ => current_pool_size, // 使用率合适，保持不变
                            };

                            debug!(
                                "Would adjust component {:?} DatePage object pool size: from {} to {}",
                                component_id, current_pool_size, new_pool_size
                            );

                            // 这里可以添加实际的调整代码，例如：
                            // let adjustment_request = AdjustmentRequest {
                            //     component_id: component_id.clone(),
                            //     adjustment_type: AdjustmentType::AdjustConcurrency,
                            //     params: serde_json::json!({
                            //         "date_page_pool_size": new_pool_size
                            //     }),
                            // };
                            // self.handle_adjustment_request(adjustment_request).await;
                        }
                        super::types::ComponentType::Crawl4Ai => {
                            // 对于Crawl4Ai组件，调整并发数
                            let current_concurrency = 10; // 这里需要实际获取当前并发数
                            let new_concurrency = match usage_diff.signum() {
                                1.0 => {
                                    // 当前使用率过高，减少并发数
                                    (current_concurrency as f64 * 0.8) as usize
                                }
                                -1.0 => {
                                    // 当前使用率过低，增加并发数
                                    (current_concurrency as f64 * 1.2) as usize
                                }
                                _ => current_concurrency, // 使用率合适，保持不变
                            };

                            debug!(
                                "Would adjust component {:?} concurrency: from {} to {}",
                                component_id, current_concurrency, new_concurrency
                            );

                            // 这里可以添加实际的调整代码
                        }
                        _ => {
                            // 其他组件，根据使用率调整资源分配
                            debug!(
                                "Would adjust component {:?} with allocation: {:.2}",
                                component_id, suggested_allocation
                            );
                        }
                    };
                }
            }
        }

        Ok(())
    }
}

/// SystemController实现ConfigUpdateHandler trait，处理配置更新事件
#[async_trait::async_trait]
impl ConfigUpdateHandler for SystemController {
    async fn handle_config_update(&self, event: ConfigUpdateEvent) -> seesea_errors::Result<()> {
        debug!("Handling config update event: {:?}", event);

        match event {
            ConfigUpdateEvent::HttpPoolUpdate {
                component_id,
                config: _config,
            } => {
                info!("Received HTTP pool update for component {:?}", component_id);
                // 这里可以添加具体的处理逻辑，例如：
                // 1. 更新组件的连接池配置
                // 2. 通知组件重新创建客户端
                // 3. 更新组件状态
                Ok(())
            }
            ConfigUpdateEvent::DnsConfigUpdate {
                component_id,
                config: _config,
            } => {
                info!(
                    "Received DNS config update for component {:?}",
                    component_id
                );
                // 处理DNS配置更新
                Ok(())
            }
            ConfigUpdateEvent::NetworkConfigUpdate { config: _config } => {
                info!("Received global network config update");
                // 处理全局网络配置更新
                Ok(())
            }
            ConfigUpdateEvent::ResourceLimitUpdate {
                component_id,
                resource_type,
                limit,
            } => {
                info!(
                    "Received resource limit update for component {:?}, resource type {:?}, limit: {}",
                    component_id, resource_type, limit
                );
                // 处理资源限制更新
                Ok(())
            }
            ConfigUpdateEvent::BatchSizeUpdate {
                component_id,
                batch_size,
            } => {
                info!(
                    "Received batch size update for component {:?}, batch size: {}",
                    component_id, batch_size
                );
                // 处理批量大小更新
                Ok(())
            }
            ConfigUpdateEvent::Other { key, value } => {
                info!(
                    "Received other config update for key: {}, value: {:?}",
                    key, value
                );
                // 处理其他配置更新
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ComponentId, ComponentType};

    #[tokio::test]
    async fn test_system_controller() {
        let config = SystemControllerConfig::default();
        let controller = SystemController::new(config);

        // 测试获取监控器和管理器
        let _resource_monitor = controller.resource_monitor();
        let _priority_manager = controller.priority_manager();
        // Arc类型不能使用is_null()或is_some()，直接测试其存在性
        // 这里我们只需要确认方法能正常调用，不需要额外断言

        // 测试注册组件
        let component_id = ComponentId::new(ComponentType::VectorStore, "test_component");
        let component_config = ComponentConfig {
            id: component_id.clone(),
            priority: 50,
            max_resource_usage: 0.8,
            min_resource_allocation: 0.1,
            enable_dynamic_adjustment: true,
            adjustment_params: serde_json::Value::Null,
        };

        controller
            .register_component(component_config)
            .await
            .unwrap();

        // 测试获取组件状态
        let component_status = controller.get_component_status(&component_id).await;
        assert!(component_status.is_some());

        // 测试更新组件状态
        let updated_status = ComponentStatus {
            component_id: component_id.clone(),
            current_resource_usage: 0.5,
            current_priority: 50,
            current_params: serde_json::Value::Null,
            healthy: true,
            last_adjustment_timestamp: 0,
        };
        controller
            .update_component_status(updated_status)
            .await
            .unwrap();

        let status_after_update = controller.get_component_status(&component_id).await;
        assert!(status_after_update.is_some());
        assert_eq!(status_after_update.unwrap().current_resource_usage, 0.5);

        // 测试获取系统状态
        let system_status = controller.get_system_status().await;
        assert_eq!(system_status.component_statuses.len(), 1);

        // 测试注销组件
        controller
            .unregister_component(&component_id)
            .await
            .unwrap();
        let status_after_remove = controller.get_component_status(&component_id).await;
        assert!(status_after_remove.is_none());
    }
}
