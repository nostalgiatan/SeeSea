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

//! 守护进程模块
//!
//! 负责监控系统状态并进行自动修复，确保系统稳定运行

use super::controller::SystemController;
use super::types::DaemonConfig;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, warn};

/// 守护进程
///
/// 监控系统状态并进行自动修复，确保系统稳定运行
pub struct Daemon {
    /// 系统控制器
    system_controller: Arc<SystemController>,
    /// 配置
    config: DaemonConfig,
    /// 守护进程是否运行
    running: Arc<RwLock<bool>>,
    /// 重启计数器
    restart_counter: Arc<RwLock<u32>>,
}

impl Clone for Daemon {
    fn clone(&self) -> Self {
        Self {
            system_controller: self.system_controller.clone(),
            config: self.config.clone(),
            running: self.running.clone(),
            restart_counter: self.restart_counter.clone(),
        }
    }
}

impl Daemon {
    /// 创建新的守护进程
    pub fn new(system_controller: Arc<SystemController>, config: DaemonConfig) -> Self {
        Self {
            system_controller,
            config,
            running: Arc::new(RwLock::new(false)),
            restart_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// 获取系统控制器
    pub fn system_controller(&self) -> Arc<SystemController> {
        self.system_controller.clone()
    }

    /// 获取配置
    pub fn config(&self) -> &DaemonConfig {
        &self.config
    }

    /// 启动守护进程
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            debug!("Daemon is already running");
            return;
        }
        *running = true;
        drop(running);

        info!("Starting daemon with config: {:?}", self.config);

        let mut interval = interval(Duration::from_millis(self.config.check_interval_ms));

        loop {
            interval.tick().await;
            match self.check_system_status().await {
                Ok(_) => {
                    debug!("System status check completed successfully");
                }
                Err(e) => {
                    warn!("Failed to check system status: {}", e);
                }
            }
        }
    }

    /// 停止守护进程
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Daemon stopped");
    }

    /// 检查系统控制器是否运行
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// 检查系统状态
    async fn check_system_status(&self) -> Result<(), String> {
        // 获取系统状态
        let system_status = self.system_controller.get_system_status().await;

        debug!("Checking system status: {:?}", system_status);

        // 检查系统控制器是否运行
        if !system_status.controller_running {
            warn!("System controller is not running, attempting to restart");
            self.restart_system_controller().await?;
        }

        // 检查组件状态
        for component_status in system_status.component_statuses.iter() {
            if !component_status.healthy {
                warn!(
                    "Component {:?} is not healthy, attempting to fix",
                    component_status.component_id
                );
                if self.config.auto_fix {
                    self.fix_unhealthy_component(component_status).await?;
                }
            }
        }

        // 检查资源状态
        self.check_resource_status(&system_status.resource_status)
            .await?;

        Ok(())
    }

    /// 检查资源状态
    async fn check_resource_status(
        &self,
        resource_status: &super::types::ResourceStatus,
    ) -> Result<(), String> {
        // 检查CPU使用率
        if resource_status.cpu_usage > 0.95 {
            // CPU使用率超过95%
            warn!(
                "CPU usage is critically high: {:.2}%",
                resource_status.cpu_usage * 100.0
            );
            if self.config.auto_fix {
                self.handle_high_cpu_usage().await?;
            }
        }

        // 检查内存使用率
        if resource_status.memory_usage > 0.95 {
            // 内存使用率超过95%
            warn!(
                "Memory usage is critically high: {:.2}%",
                resource_status.memory_usage * 100.0
            );
            if self.config.auto_fix {
                self.handle_high_memory_usage().await?;
            }
        }

        // 检查磁盘空间
        if resource_status.available_disk < 10 * 1024 * 1024 * 1024 {
            // 可用磁盘空间少于10GB
            warn!(
                "Available disk space is critically low: {} bytes",
                resource_status.available_disk
            );
            if self.config.auto_fix {
                self.handle_low_disk_space().await?;
            }
        }

        // 检查系统负载
        let cpu_count = num_cpus::get() as f64;
        if resource_status.load_avg_1 > cpu_count * 2.0 {
            // 1分钟负载超过CPU核心数的2倍
            warn!(
                "System load is critically high: {:.2} (CPU count: {})",
                resource_status.load_avg_1, cpu_count
            );
            if self.config.auto_fix {
                self.handle_high_system_load().await?;
            }
        }

        Ok(())
    }

    /// 重启系统控制器
    async fn restart_system_controller(&self) -> Result<(), String> {
        let mut restart_counter = self.restart_counter.write().await;

        if *restart_counter >= self.config.max_restart_attempts {
            return Err(format!(
                "Max restart attempts ({}) reached, cannot restart system controller",
                self.config.max_restart_attempts
            ));
        }

        info!(
            "Restarting system controller (attempt {}/{})",
            *restart_counter + 1,
            self.config.max_restart_attempts
        );

        // 停止系统控制器
        self.system_controller.stop().await;

        // 等待一段时间
        tokio::time::sleep(Duration::from_millis(self.config.restart_delay_ms)).await;

        // 启动系统控制器
        let system_controller_clone = self.system_controller.clone();
        spawn(async move {
            system_controller_clone.start().await;
        });

        // 增加重启计数器
        *restart_counter += 1;

        Ok(())
    }

    /// 修复不健康的组件
    async fn fix_unhealthy_component(
        &self,
        _component_status: &super::types::ComponentStatus,
    ) -> Result<(), String> {
        // 这里可以添加具体的组件修复逻辑
        // 例如：
        // - 重启组件
        // - 重新初始化组件
        // - 调整组件参数
        // - 记录详细日志

        debug!(
            "Fixing unhealthy component: {:?}",
            _component_status.component_id
        );

        // 目前仅记录日志，实际修复逻辑需要根据具体组件实现
        Ok(())
    }

    /// 处理高CPU使用率
    async fn handle_high_cpu_usage(&self) -> Result<(), String> {
        // 这里可以添加处理高CPU使用率的逻辑
        // 例如：
        // - 降低组件的资源分配
        // - 减少批量大小
        // - 降低线程数
        // - 暂停非关键任务

        debug!("Handling high CPU usage");

        // 目前仅记录日志，实际处理逻辑需要根据具体情况实现
        Ok(())
    }

    /// 处理高内存使用率
    async fn handle_high_memory_usage(&self) -> Result<(), String> {
        // 这里可以添加处理高内存使用率的逻辑
        // 例如：
        // - 清理缓存
        // - 降低组件的内存限制
        // - 减少批量大小
        // - 暂停非关键任务

        debug!("Handling high memory usage");

        // 目前仅记录日志，实际处理逻辑需要根据具体情况实现
        Ok(())
    }

    /// 处理低磁盘空间
    async fn handle_low_disk_space(&self) -> Result<(), String> {
        // 这里可以添加处理低磁盘空间的逻辑
        // 例如：
        // - 清理临时文件
        // - 清理过期的缓存
        // - 提醒用户
        // - 暂停写入操作

        debug!("Handling low disk space");

        // 目前仅记录日志，实际处理逻辑需要根据具体情况实现
        Ok(())
    }

    /// 处理高系统负载
    async fn handle_high_system_load(&self) -> Result<(), String> {
        // 这里可以添加处理高系统负载的逻辑
        // 例如：
        // - 降低组件的资源分配
        // - 减少批量大小
        // - 降低线程数
        // - 暂停非关键任务

        debug!("Handling high system load");

        // 目前仅记录日志，实际处理逻辑需要根据具体情况实现
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SystemControllerConfig;
    use std::time::Duration;

    #[tokio::test]
    async fn test_daemon() {
        // 创建系统控制器
        let system_controller_config = SystemControllerConfig::default();
        let system_controller = Arc::new(SystemController::new(system_controller_config));

        // 创建守护进程
        let daemon_config = DaemonConfig::default();
        let daemon = Daemon::new(system_controller.clone(), daemon_config.clone());

        // 测试获取系统控制器和配置
        let _retrieved_controller = daemon.system_controller();
        let retrieved_config = daemon.config();
        // Arc类型不能使用is_null()或is_some()，直接测试其存在性
        // 这里我们只需要确认方法能正常调用，不需要额外断言
        assert_eq!(
            retrieved_config.check_interval_ms,
            daemon_config.check_interval_ms
        );

        // 测试检查系统状态
        let system_status = system_controller.get_system_status().await;
        assert!(!system_status.controller_running);

        // 测试启动和停止
        let daemon_clone = daemon.clone();
        let handle = tokio::spawn(async move {
            daemon_clone.start().await;
        });

        // 等待守护进程启动
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(daemon.is_running().await);

        // 停止守护进程
        daemon.stop().await;
        assert!(!daemon.is_running().await);

        // 取消任务
        handle.abort();
    }
}
