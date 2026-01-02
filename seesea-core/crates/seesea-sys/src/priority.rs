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

//! 优先级管理器模块
//!
//! 负责管理各个组件的优先级和资源分配

use super::types::{
    AdjustmentRequest, AdjustmentResponse, ComponentConfig, ComponentId, ResourceStatus,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// 优先级管理器
///
/// 管理各个组件的优先级和资源分配，根据组件优先级和当前资源状态计算资源分配
pub struct PriorityManager {
    /// 组件配置映射
    component_configs: Arc<RwLock<HashMap<ComponentId, ComponentConfig>>>,
    /// 优先级调整因子
    priority_factor: f64,
}

impl PriorityManager {
    /// 创建新的优先级管理器
    pub fn new(priority_factor: f64) -> Self {
        Self {
            component_configs: Arc::new(RwLock::new(HashMap::new())),
            priority_factor,
        }
    }

    /// 注册组件
    pub async fn register_component(&self, config: ComponentConfig) -> Result<(), String> {
        let mut configs = self.component_configs.write().await;
        if configs.contains_key(&config.id) {
            return Err(format!("Component already registered: {:?}", config.id));
        }
        configs.insert(config.id.clone(), config.clone());
        debug!(
            "Component registered: {:?}, priority: {}",
            config.id, config.priority
        );
        Ok(())
    }

    /// 注销组件
    pub async fn unregister_component(&self, component_id: &ComponentId) -> Result<(), String> {
        let mut configs = self.component_configs.write().await;
        if let Some(config) = configs.remove(component_id) {
            debug!("Component unregistered: {:?}", config.id);
            Ok(())
        } else {
            Err(format!("Component not found: {component_id:?}"))
        }
    }

    /// 获取组件配置
    pub async fn get_component_config(
        &self,
        component_id: &ComponentId,
    ) -> Result<Option<ComponentConfig>, String> {
        let configs = self.component_configs.read().await;
        Ok(configs.get(component_id).cloned())
    }

    /// 更新组件配置
    pub async fn update_component_config(&self, config: ComponentConfig) -> Result<(), String> {
        let mut configs = self.component_configs.write().await;
        let old_config = configs.insert(config.id.clone(), config.clone());
        debug!(
            "Component config updated: {:?}, old_priority: {:?}, new_priority: {}",
            config.id,
            old_config.map(|c| c.priority),
            config.priority
        );
        Ok(())
    }

    /// 调整组件优先级
    pub async fn adjust_priority(
        &self,
        component_id: &ComponentId,
        new_priority: u8,
    ) -> Result<(), String> {
        let mut configs = self.component_configs.write().await;
        if let Some(config) = configs.get_mut(component_id) {
            let old_priority = config.priority;
            config.priority = new_priority;
            debug!(
                "Component priority adjusted: {:?}, old: {}, new: {}",
                component_id, old_priority, new_priority
            );
            Ok(())
        } else {
            Err(format!("Component not found: {component_id:?}"))
        }
    }

    /// 根据资源状态计算资源分配
    ///
    /// 返回一个映射，键为组件ID，值为该组件应该获得的资源份额（0.0-1.0）
    pub async fn calculate_resource_allocation(
        &self,
        _resource_status: &ResourceStatus,
    ) -> HashMap<ComponentId, f64> {
        let configs = self.component_configs.read().await;

        // 如果没有组件，返回空映射
        if configs.is_empty() {
            return HashMap::new();
        }

        // 计算总优先级
        let total_priority: f64 = configs.values().map(|config| config.priority as f64).sum();

        // 如果总优先级为0，返回平均分配
        if total_priority == 0.0 {
            let equal_share = 1.0 / configs.len() as f64;
            configs.keys().map(|id| (id.clone(), equal_share)).collect()
        } else {
            // 计算每个组件的资源份额
            configs
                .iter()
                .map(|(id, config)| {
                    // 基础份额：根据优先级计算
                    let base_share =
                        (config.priority as f64 / total_priority) * self.priority_factor;

                    // 剩余份额：平均分配
                    let remaining_share = (1.0 - self.priority_factor) / configs.len() as f64;

                    // 总份额
                    let total_share = base_share + remaining_share;

                    // 确保份额在最小和最大限制之间
                    let bounded_share = total_share
                        .max(config.min_resource_allocation)
                        .min(config.max_resource_usage);

                    (id.clone(), bounded_share)
                })
                .collect()
        }
    }

    /// 处理调整请求
    pub async fn handle_adjustment_request(
        &self,
        request: AdjustmentRequest,
    ) -> AdjustmentResponse {
        match request.adjustment_type {
            super::types::AdjustmentType::AdjustPriority => {
                // 调整优先级
                let new_priority = request
                    .params
                    .get("priority")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u8)
                    .ok_or_else(|| "Invalid priority value".to_string());

                match new_priority {
                    Ok(priority) => {
                        match self.adjust_priority(&request.component_id, priority).await {
                            Ok(_) => AdjustmentResponse {
                                success: true,
                                adjusted_params: serde_json::json!({ "priority": priority }),
                                reason: format!("Priority adjusted to {priority}"),
                            },
                            Err(e) => AdjustmentResponse {
                                success: false,
                                adjusted_params: serde_json::Value::Null,
                                reason: e,
                            },
                        }
                    }
                    Err(e) => AdjustmentResponse {
                        success: false,
                        adjusted_params: serde_json::Value::Null,
                        reason: e,
                    },
                }
            }
            super::types::AdjustmentType::AdjustConcurrency => {
                // 调整并发数量
                let new_concurrency = request
                    .params
                    .get("concurrency")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .ok_or_else(|| "Invalid concurrency value".to_string());

                match new_concurrency {
                    Ok(concurrency) => {
                        // 对于并发数量调整，我们只需要记录并返回成功
                        // 具体的并发调整逻辑由组件自身实现
                        AdjustmentResponse {
                            success: true,
                            adjusted_params: serde_json::json!({ "concurrency": concurrency }),
                            reason: format!("Concurrency adjusted to {concurrency}"),
                        }
                    }
                    Err(e) => AdjustmentResponse {
                        success: false,
                        adjusted_params: serde_json::Value::Null,
                        reason: e,
                    },
                }
            }
            _ => AdjustmentResponse {
                success: false,
                adjusted_params: serde_json::Value::Null,
                reason: "Adjustment type not supported".to_string(),
            },
        }
    }

    /// 获取所有组件配置
    pub async fn get_all_components(&self) -> Vec<ComponentConfig> {
        let configs = self.component_configs.read().await;
        configs.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ComponentId, ComponentType};

    #[tokio::test]
    async fn test_priority_manager() {
        let manager = PriorityManager::new(0.5);

        // 创建组件配置
        let config1 = ComponentConfig {
            id: ComponentId::new(ComponentType::VectorStore, "test1"),
            priority: 100,
            max_resource_usage: 0.8,
            min_resource_allocation: 0.1,
            enable_dynamic_adjustment: true,
            adjustment_params: serde_json::Value::Null,
        };

        let config2 = ComponentConfig {
            id: ComponentId::new(ComponentType::Cache, "test2"),
            priority: 50,
            max_resource_usage: 0.8,
            min_resource_allocation: 0.1,
            enable_dynamic_adjustment: true,
            adjustment_params: serde_json::Value::Null,
        };

        // 注册组件
        manager.register_component(config1.clone()).await.unwrap();
        manager.register_component(config2.clone()).await.unwrap();

        // 测试获取组件配置
        let retrieved_config1 = manager.get_component_config(&config1.id).await.unwrap();
        assert!(retrieved_config1.is_some());
        assert_eq!(retrieved_config1.unwrap().priority, 100);

        // 测试调整优先级
        manager.adjust_priority(&config1.id, 80).await.unwrap();
        let updated_config1 = manager.get_component_config(&config1.id).await.unwrap();
        assert!(updated_config1.is_some());
        assert_eq!(updated_config1.unwrap().priority, 80);

        // 测试计算资源分配
        let resource_status = ResourceStatus::default();
        let allocation = manager
            .calculate_resource_allocation(&resource_status)
            .await;
        assert_eq!(allocation.len(), 2);

        // 组件1的优先级更高，应该获得更多资源
        let share1 = allocation.get(&config1.id).unwrap();
        let share2 = allocation.get(&config2.id).unwrap();
        assert!(*share1 > *share2);

        // 注销组件
        manager.unregister_component(&config1.id).await.unwrap();
        let allocation_after_remove = manager
            .calculate_resource_allocation(&resource_status)
            .await;
        assert_eq!(allocation_after_remove.len(), 1);
    }
}
