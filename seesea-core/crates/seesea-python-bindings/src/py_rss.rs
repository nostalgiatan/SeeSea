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

//! Python bindings for RSS functionality

use pyo3::IntoPyObjectExt;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use tokio::sync::RwLock;

use seesea_api::static_files::get_rss_template_dir;
use seesea_config::NetworkConfig;
use seesea_derive::RssFeedQuery;
use seesea_net::client::HttpClient;
use seesea_rss::RssInterface;

// 使用 py_cache.rs 中定义的全局缓存实例
use crate::py_cache::GLOBAL_CACHE_INSTANCE;

#[pyclass]
pub struct PyRssClient {
    runtime: tokio::runtime::Runtime,
    interface: Arc<RwLock<RssInterface>>,
}

#[pymethods]
impl PyRssClient {
    #[new]
    pub fn new() -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create runtime: {e}"
            ))
        })?;

        let interface = {
            let network_config = NetworkConfig::default();
            let client = Arc::new(HttpClient::new(network_config).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to create HTTP client: {e}"
                ))
            })?);

            // 使用全局缓存实例
            let rss_cache = Arc::new(RwLock::new(GLOBAL_CACHE_INSTANCE.rss()));

            let mut rss_interface = RssInterface::with_cache(client, rss_cache);

            // 使用统一的资源路径查找
            let template_dir = get_rss_template_dir();
            let _ = rss_interface.set_template_dir(template_dir.to_string_lossy().as_ref());
            tracing::debug!("RSS template directory set to: {}", template_dir.display());

            Arc::new(RwLock::new(rss_interface))
        };

        Ok(Self { runtime, interface })
    }

    /// 获取 RSS feed
    pub fn fetch_feed(
        &self,
        url: String,
        max_items: Option<usize>,
        filter_keywords: Option<Vec<String>>,
    ) -> PyResult<Py<PyAny>> {
        let query = RssFeedQuery {
            url,
            max_items,
            filter_keywords: filter_keywords.unwrap_or_default(),
            after_date: None,
        };

        let feed = self
            .runtime
            .block_on(async {
                let interface = self.interface.read().await;
                interface.fetch(&query).await
            })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to fetch RSS feed: {e}"
                ))
            })?;

        Python::attach(|py| {
            let dict = PyDict::new(py);

            // 添加元数据
            let meta = PyDict::new(py);
            meta.set_item("title", &feed.meta.title)?;
            meta.set_item("link", &feed.meta.link)?;
            meta.set_item("description", &feed.meta.description)?;
            meta.set_item("language", &feed.meta.language)?;
            dict.set_item("meta", meta)?;

            // 添加项目
            let items: Vec<Py<PyAny>> = feed
                .items
                .iter()
                .map(|item| {
                    let item_dict = PyDict::new(py);
                    let _ = item_dict.set_item("title", &item.title);
                    let _ = item_dict.set_item("link", &item.link);
                    let _ = item_dict.set_item("description", &item.description);
                    let _ = item_dict.set_item("author", &item.author);
                    let _ = item_dict.set_item("pub_date", &item.pub_date);
                    let _ = item_dict.set_item("content", &item.content);
                    let _ = item_dict.set_item("categories", &item.categories);
                    item_dict.into_py_any(py).unwrap_or_else(|_| py.None())
                })
                .collect();

            dict.set_item("items", items)?;
            dict.into_py_any(py)
        })
    }

    /// 解析 RSS feed 内容
    pub fn parse_feed(&self, content: String) -> PyResult<Py<PyAny>> {
        let feed = self
            .runtime
            .block_on(async {
                let interface = self.interface.read().await;
                interface.parse(&content)
            })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to parse RSS feed: {e}"
                ))
            })?;

        Python::attach(|py| {
            let dict = PyDict::new(py);

            // 添加元数据
            let meta = PyDict::new(py);
            meta.set_item("title", &feed.meta.title)?;
            meta.set_item("link", &feed.meta.link)?;
            meta.set_item("description", &feed.meta.description)?;
            dict.set_item("meta", meta)?;

            // 添加项目
            let items: Vec<Py<PyAny>> = feed
                .items
                .iter()
                .map(|item| {
                    let item_dict = PyDict::new(py);
                    let _ = item_dict.set_item("title", &item.title);
                    let _ = item_dict.set_item("link", &item.link);
                    let _ = item_dict.set_item("description", &item.description);
                    item_dict.into_py_any(py).unwrap_or_else(|_| py.None())
                })
                .collect();

            dict.set_item("items", items)?;
            dict.into_py_any(py)
        })
    }

    /// 列出可用的模板
    pub fn list_templates(&self) -> PyResult<Vec<String>> {
        let templates = self
            .runtime
            .block_on(async {
                let interface = self.interface.read().await;
                interface.list_templates()
            })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to list templates: {e}"
                ))
            })?;

        Ok(templates)
    }

    /// 从模板添加 RSS feeds
    pub fn add_from_template(
        &self,
        template_name: String,
        categories: Option<Vec<String>>,
    ) -> PyResult<usize> {
        let count = self
            .runtime
            .block_on(async {
                let interface = self.interface.read().await;
                interface
                    .add_from_template(&template_name, categories)
                    .await
            })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to add feeds from template: {e}"
                ))
            })?;

        Ok(count)
    }

    /// 创建RSS榜单 - 基于关键词对RSS项目进行评分和排名
    ///
    /// # Arguments
    ///
    /// * `feed_urls` - RSS Feed URL列表
    /// * `keywords` - 关键词及权重列表 [(keyword, weight), ...]
    /// * `min_score` - 最小评分阈值
    /// * `max_results` - 最大结果数
    ///
    /// # Returns
    ///
    /// 返回排序后的RSS榜单
    pub fn create_ranking(
        &self,
        feed_urls: Vec<String>,
        keywords: Vec<(String, f64)>,
        min_score: Option<f64>,
        max_results: Option<usize>,
    ) -> PyResult<Py<PyAny>> {
        use seesea_rss::ranking::{RankingConfig, RankingKeyword, RssRankingEngine};

        // 构建关键词配置
        let kw_configs: Vec<RankingKeyword> = keywords
            .into_iter()
            .map(|(kw, weight)| RankingKeyword::new(kw, weight))
            .collect();

        let config = RankingConfig {
            name: "python_ranking".to_string(),
            keywords: kw_configs,
            min_score: min_score.unwrap_or(0.0),
            max_results: max_results.unwrap_or(100),
        };

        // 获取所有 feeds
        let feeds = self.runtime.block_on(async {
            let interface = self.interface.read().await;
            let mut all_feeds = Vec::new();

            for url in feed_urls {
                let query = RssFeedQuery {
                    url,
                    max_items: None,
                    filter_keywords: vec![],
                    after_date: None,
                };

                match interface.fetch(&query).await {
                    Ok(feed) => all_feeds.push(feed),
                    Err(_) => continue, // 跳过失败的 feed
                }
            }

            all_feeds
        });

        // 创建榜单引擎并评分
        let engine = RssRankingEngine::new(config).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create ranking engine: {e}"
            ))
        })?;
        let ranking = engine.rank_feeds(&feeds).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to rank feeds: {e}"))
        })?;

        // 转换为 Python 对象
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("name", ranking.name)?;
            dict.set_item("total_items", ranking.total_items)?;
            dict.set_item("timestamp", ranking.timestamp)?;

            let items: Vec<Py<PyAny>> = ranking
                .items
                .iter()
                .map(|scored_item| {
                    let item_dict = PyDict::new(py);
                    let _ = item_dict.set_item("title", &scored_item.item.title);
                    let _ = item_dict.set_item("link", &scored_item.item.link);
                    let _ = item_dict.set_item("description", &scored_item.item.description);
                    let _ = item_dict.set_item("pub_date", &scored_item.item.pub_date);
                    let _ = item_dict.set_item("score", scored_item.score);
                    let _ = item_dict.set_item("matched_keywords", &scored_item.matched_keywords);
                    item_dict.into_py_any(py).unwrap_or_else(|e| {
                        tracing::warn!("Failed to convert RSS item to Python: {}", e);
                        py.None()
                    })
                })
                .collect();

            dict.set_item("items", items)?;
            dict.into_py_any(py)
        })
    }
}
