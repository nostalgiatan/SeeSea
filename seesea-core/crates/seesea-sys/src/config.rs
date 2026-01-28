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

//! 动态配置管理模块
//!
//! 负责管理系统的动态配置，支持实时更新和传播配置变更

use crate::types::{ComponentId, ResourceType};
use seesea_config::network::{DnsConfig, NetworkConfig, PoolConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast::{Receiver, Sender, channel};
use tracing::{error, info};

/// 配置更新事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigUpdateEvent {
    /// HTTP连接池配置更新
    HttpPoolUpdate {
        /// 组件ID（可选，None表示全局更新）
        component_id: Option<ComponentId>,
        /// 新的连接池配置
        config: PoolConfig,
    },
    /// DNS配置更新
    DnsConfigUpdate {
        /// 组件ID（可选，None表示全局更新）
        component_id: Option<ComponentId>,
        /// 新的DNS配置
        config: DnsConfig,
    },
    /// 全局网络配置更新
    NetworkConfigUpdate {
        /// 新的网络配置
        config: Box<NetworkConfig>,
    },
    /// 资源限制更新
    ResourceLimitUpdate {
        /// 组件ID
        component_id: ComponentId,
        /// 资源类型
        resource_type: ResourceType,
        /// 新的限制值（0.0-1.0）
        limit: f64,
    },
    /// 批量大小更新
    BatchSizeUpdate {
        /// 组件ID
        component_id: ComponentId,
        /// 新的批量大小
        batch_size: usize,
    },
    /// 其他配置更新
    Other {
        /// 配置键
        key: String,
        /// 配置值
        value: serde_json::Value,
    },
}

/// 动态配置管理器
///
/// 管理系统的动态配置，支持配置更新和事件传播
pub struct DynamicConfigManager {
    /// 当前网络配置
    current_config: Arc<RwLock<NetworkConfig>>,
    /// 配置更新事件发送器
    event_sender: Sender<ConfigUpdateEvent>,
    /// 配置更新事件接收器
    _event_receiver: Receiver<ConfigUpdateEvent>,
}

impl DynamicConfigManager {
    /// 创建新的动态配置管理器
    pub fn new(initial_config: NetworkConfig) -> Self {
        // 创建广播通道，容量为100
        let (sender, receiver) = channel(100);

        Self {
            current_config: Arc::new(RwLock::new(initial_config)),
            event_sender: sender,
            _event_receiver: receiver,
        }
    }

    /// 获取当前网络配置
    pub async fn get_current_config(&self) -> NetworkConfig {
        self.current_config.read().await.clone()
    }

    /// 获取配置更新事件接收器
    pub fn subscribe(&self) -> Receiver<ConfigUpdateEvent> {
        self.event_sender.subscribe()
    }

    /// 更新HTTP连接池配置
    pub async fn update_http_pool_config(
        &self,
        component_id: Option<ComponentId>,
        config: PoolConfig,
    ) -> seesea_errors::Result<()> {
        // 更新当前配置
        let mut current_config = self.current_config.write().await;
        current_config.pool = config.clone();
        drop(current_config);

        // 发送配置更新事件
        let event = ConfigUpdateEvent::HttpPoolUpdate {
            component_id,
            config,
        };

        match self.event_sender.send(event) {
            Ok(_) => {
                info!("HTTP pool config updated, event sent to subscribers");
                Ok(())
            }
            Err(e) => {
                error!("Failed to send HTTP pool config update event: {e}",);
                Err(seesea_errors::business_error(format!(
                    "Failed to send config update event: {e}",
                )))
            }
        }
    }

    /// 更新DNS配置
    pub async fn update_dns_config(
        &self,
        component_id: Option<ComponentId>,
        config: DnsConfig,
    ) -> seesea_errors::Result<()> {
        // 更新当前配置
        let mut current_config = self.current_config.write().await;
        current_config.dns = config.clone();
        drop(current_config);

        // 发送配置更新事件
        let event = ConfigUpdateEvent::DnsConfigUpdate {
            component_id,
            config,
        };

        match self.event_sender.send(event) {
            Ok(_) => {
                info!("DNS config updated, event sent to subscribers");
                Ok(())
            }
            Err(e) => {
                error!("Failed to send DNS config update event: {e}",);
                Err(seesea_errors::business_error(format!(
                    "Failed to send config update event: {e}",
                )))
            }
        }
    }

    /// 更新全局网络配置
    pub async fn update_network_config(&self, config: NetworkConfig) -> seesea_errors::Result<()> {
        // 更新当前配置
        *self.current_config.write().await = config.clone();

        // 发送配置更新事件
        let event = ConfigUpdateEvent::NetworkConfigUpdate {
            config: Box::new(config),
        };

        match self.event_sender.send(event) {
            Ok(_) => {
                info!("Network config updated, event sent to subscribers");
                Ok(())
            }
            Err(e) => {
                error!("Failed to send network config update event: {e}",);
                Err(seesea_errors::business_error(format!(
                    "Failed to send config update event: {e}",
                )))
            }
        }
    }

    /// 更新资源限制
    pub async fn update_resource_limit(
        &self,
        component_id: ComponentId,
        resource_type: ResourceType,
        limit: f64,
    ) -> seesea_errors::Result<()> {
        // 先记录日志，然后再移动值
        info!(
            "Resource limit updated for component {:?}, event sent to subscribers",
            component_id
        );

        // 发送配置更新事件
        let event = ConfigUpdateEvent::ResourceLimitUpdate {
            component_id,
            resource_type,
            limit,
        };

        match self.event_sender.send(event) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to send resource limit update event: {e}",);
                Err(seesea_errors::business_error(format!(
                    "Failed to send config update event: {e}",
                )))
            }
        }
    }

    /// 更新批量大小
    pub async fn update_batch_size(
        &self,
        component_id: ComponentId,
        batch_size: usize,
    ) -> seesea_errors::Result<()> {
        // 先记录日志，然后再移动值
        info!(
            "Batch size updated for component {:?}, event sent to subscribers",
            component_id
        );

        // 发送配置更新事件
        let event = ConfigUpdateEvent::BatchSizeUpdate {
            component_id,
            batch_size,
        };

        match self.event_sender.send(event) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to send batch size update event: {e}",);
                Err(seesea_errors::business_error(format!(
                    "Failed to send config update event: {e}",
                )))
            }
        }
    }

    /// 更新其他配置
    pub async fn update_other_config(
        &self,
        key: String,
        value: serde_json::Value,
    ) -> seesea_errors::Result<()> {
        // 先记录日志，然后再移动值
        info!(
            "Other config updated for key {}, event sent to subscribers",
            key
        );

        // 发送配置更新事件
        let event = ConfigUpdateEvent::Other { key, value };

        match self.event_sender.send(event) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to send other config update event: {e}",);
                Err(seesea_errors::business_error(format!(
                    "Failed to send config update event: {e}",
                )))
            }
        }
    }
}

/// 配置更新处理器
///
/// 组件可以实现此trait来处理配置更新事件
#[async_trait::async_trait]
pub trait ConfigUpdateHandler {
    /// 处理配置更新事件
    async fn handle_config_update(&self, event: ConfigUpdateEvent) -> seesea_errors::Result<()>;
}
