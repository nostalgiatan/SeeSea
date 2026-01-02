// Copyright (C) 2024 SeeSea Authors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! 蚁群算法实现
//! 实现基于蚁群算法的相关性分析，用于数据清洗

use crate::DataBlock;
use crate::simd_utils::simd_cosine_similarity;

/// 蚁群算法配置
pub struct AntColonyConfig {
    /// 信息素蒸发率
    pub pheromone_evaporation: f32,
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 收敛阈值
    pub convergence_threshold: f32,
    /// 标题权重
    pub title_weight: f32,
    /// 是否启用自适应权重调整
    pub adaptive_weight: bool,
    /// 是否启用自适应信息素蒸发率调整
    pub adaptive_evaporation: bool,
    /// 是否启用自适应top_k_similar调整
    pub adaptive_top_k: bool,
    /// 是否启用异常值处理
    pub handle_outliers: bool,
    /// 最大相似数据块数量
    pub max_similar_blocks: usize,
    /// 只计算与当前数据块最相似的k个数据块
    pub top_k_similar: usize,
}

impl Default for AntColonyConfig {
    fn default() -> Self {
        Self {
            pheromone_evaporation: 0.1,
            max_iterations: 10,
            convergence_threshold: 0.001,
            title_weight: 0.7,
            adaptive_weight: true,
            adaptive_evaporation: true,
            adaptive_top_k: true,
            handle_outliers: true,
            max_similar_blocks: 50,
            top_k_similar: 20,
        }
    }
}

/// 蚁群算法实现
pub struct AntColonyOptimization {
    config: AntColonyConfig,
}

impl Default for AntColonyOptimization {
    fn default() -> Self {
        Self::new(AntColonyConfig::default())
    }
}

impl AntColonyOptimization {
    /// 创建新的蚁群算法实例
    pub fn new(config: AntColonyConfig) -> Self {
        Self { config }
    }

    /// 计算两个向量的余弦相似度
    pub fn compute_similarity(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        simd_cosine_similarity(vec1, vec2)
    }

    /// 计算相似度矩阵
    pub fn compute_similarity_matrix(&self, blocks: &[DataBlock]) -> Vec<Vec<f32>> {
        let num_blocks = blocks.len();
        let mut similarity_matrix = vec![vec![0.0; num_blocks]; num_blocks];

        // 计算相似度矩阵的上三角
        for i in 0..num_blocks {
            similarity_matrix[i][i] = 1.0; // 自己与自己的相似度为1.0

            for j in (i + 1)..num_blocks {
                if let (
                    Some(title_vec_i),
                    Some(title_vec_j),
                    Some(content_vec_i),
                    Some(content_vec_j),
                ) = (
                    blocks[i].title_vector.as_ref(),
                    blocks[j].title_vector.as_ref(),
                    blocks[i].content_vector.as_ref(),
                    blocks[j].content_vector.as_ref(),
                ) {
                    let title_sim = self.compute_similarity(title_vec_i, title_vec_j);
                    let content_sim = self.compute_similarity(content_vec_i, content_vec_j);

                    let similarity = self.config.title_weight * title_sim
                        + (1.0 - self.config.title_weight) * content_sim;
                    similarity_matrix[i][j] = similarity;
                    similarity_matrix[j][i] = similarity; // 对称性
                }
            }
        }

        similarity_matrix
    }

    /// 自适应调整信息素蒸发率
    pub fn adapt_pheromone_evaporation(&self, iteration: usize, max_iterations: usize) -> f32 {
        // 初期高蒸发率，加快收敛；后期低蒸发率，精细调整
        // 蒸发率范围：0.05 - 0.2
        let evaporation_rate = 0.2 - (0.15 * (iteration as f32 / max_iterations as f32));
        evaporation_rate.clamp(0.05, 0.2)
    }

    /// 自适应调整top_k_similar
    pub fn adapt_top_k_similar(
        &self,
        iteration: usize,
        max_iterations: usize,
        num_blocks: usize,
    ) -> usize {
        // 初期使用较小的k值，加快收敛；后期使用较大的k值，精细调整
        // 同时考虑数据块数量，数据块越多，k值越大
        let base_k = (num_blocks / 10).clamp(5, 20);
        let adaptive_factor = 1.0 + (iteration as f32 / max_iterations as f32);

        (base_k as f32 * adaptive_factor).ceil() as usize
    }

    /// 检测和处理异常值
    pub fn detect_and_handle_outliers(&self, pheromone: &mut [f32]) {
        if pheromone.is_empty() {
            return;
        }

        // 计算四分位数
        let mut sorted_pheromone = pheromone.to_owned();
        sorted_pheromone.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let q1 = sorted_pheromone[sorted_pheromone.len() / 4];
        let q3 = sorted_pheromone[sorted_pheromone.len() * 3 / 4];
        let iqr = q3 - q1;

        // 检测异常值
        let lower_bound = q1 - 1.5 * iqr;
        let upper_bound = q3 + 1.5 * iqr;

        // 处理异常值
        for p in pheromone.iter_mut() {
            if *p < lower_bound {
                *p = lower_bound;
            } else if *p > upper_bound {
                *p = upper_bound;
            }
        }
    }

    /// 蚁群算法优化数据块得分
    pub async fn optimize_scores(&self, blocks: &mut [DataBlock]) {
        let num_blocks = blocks.len();
        if num_blocks <= 1 {
            return;
        }

        // 计算相似度矩阵
        let similarity_matrix = self.compute_similarity_matrix(blocks);

        // 初始化信息素矩阵
        let mut pheromone = vec![1.0; num_blocks];

        // 记录最佳得分和对应信息素
        let mut best_score = 0.0;
        let mut best_pheromone = pheromone.clone();

        // 主迭代循环
        for iteration in 0..self.config.max_iterations {
            // 自适应调整参数
            let pheromone_evaporation = if self.config.adaptive_evaporation {
                self.adapt_pheromone_evaporation(iteration, self.config.max_iterations)
            } else {
                self.config.pheromone_evaporation
            };

            let top_k_similar = if self.config.adaptive_top_k {
                self.adapt_top_k_similar(iteration, self.config.max_iterations, num_blocks)
            } else {
                self.config.top_k_similar
            };

            // 蚂蚁移动和信息素更新
            for (i, _) in similarity_matrix.iter().enumerate().take(num_blocks) {
                // 计算选择概率
                let mut probabilities = vec![0.0; num_blocks];
                let mut total_probability = 0.0;

                // 只考虑与当前数据块最相似的top_k_similar个数据块
                let mut similar_indices =
                    (0..num_blocks).filter(|&j| i != j).collect::<Vec<usize>>();

                // 按相似度排序，选择最相似的top_k_similar个数据块
                similar_indices.sort_by(|&a, &b| {
                    similarity_matrix[i][a]
                        .partial_cmp(&similarity_matrix[i][b])
                        .unwrap()
                        .reverse()
                });
                similar_indices.truncate(top_k_similar);

                // 计算总概率
                for &j in &similar_indices {
                    let similarity = similarity_matrix[i][j];
                    let pheromone_j = pheromone[j];
                    probabilities[j] = similarity * pheromone_j;
                    total_probability += probabilities[j];
                }

                // 归一化概率
                if total_probability > 0.0 {
                    for prob in &mut probabilities {
                        *prob /= total_probability;
                    }
                }

                // 选择下一个数据块（这里简化为随机选择）
                // 实际实现中应该根据概率分布选择
                let next_block = similar_indices.first().copied().unwrap_or(i);

                // 更新信息素
                pheromone[next_block] = pheromone[next_block] * (1.0 - pheromone_evaporation) + 1.0;
            }

            // 信息素蒸发
            for p in &mut pheromone {
                *p *= 1.0 - self.config.pheromone_evaporation;
            }

            // 异常值处理
            if self.config.handle_outliers {
                self.detect_and_handle_outliers(&mut pheromone);
            }

            // 计算当前得分
            let current_score = pheromone.iter().sum::<f32>();

            // 更新最佳得分和对应信息素
            if current_score > best_score {
                best_score = current_score;
                best_pheromone = pheromone.clone();
            }

            // 检查收敛条件
            if iteration > 0 {
                let score_diff = (current_score - best_score).abs() / best_score;
                if score_diff < self.config.convergence_threshold {
                    break;
                }
            }
        }

        // 使用最佳信息素更新数据块得分
        for i in 0..num_blocks {
            let score = best_pheromone[i] * 10.0; // 缩放得分范围
            blocks[i].score = score.min(10.0);
        }
    }
}
