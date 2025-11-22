// Copyright 2025 nostalgiatan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! 指纹对抗模块
//!
//! 提供浏览器指纹识别的对抗功能

use crate::net::types::TlsFingerprintLevel;
use rand::Rng;

/// 指纹保护器
pub struct FingerprintProtector {
    /// 指纹混淆级别
    level: TlsFingerprintLevel,
}

impl FingerprintProtector {
    /// 创建新的指纹保护器
    ///
    /// # 参数
    ///
    /// * `level` - 指纹混淆级别
    pub fn new(level: TlsFingerprintLevel) -> Self {
        Self { level }
    }

    /// 获取混淆后的 TLS 参数
    pub fn get_obfuscated_params(&self) -> ObfuscatedTlsParams {
        match self.level {
            TlsFingerprintLevel::None => ObfuscatedTlsParams::default(),
            TlsFingerprintLevel::Basic => self.apply_basic_obfuscation(),
            TlsFingerprintLevel::Advanced => self.apply_advanced_obfuscation(),
            TlsFingerprintLevel::Full => self.apply_full_obfuscation(),
        }
    }

    fn apply_basic_obfuscation(&self) -> ObfuscatedTlsParams {
        ObfuscatedTlsParams {
            cipher_suites: vec![
                "TLS_AES_128_GCM_SHA256".to_string(),
                "TLS_AES_256_GCM_SHA384".to_string(),
            ],
            supported_versions: vec!["1.3".to_string(), "1.2".to_string()],
            compression_methods: vec![],
        }
    }

    fn apply_advanced_obfuscation(&self) -> ObfuscatedTlsParams {
        // 模拟现代浏览器的 TLS 配置
        ObfuscatedTlsParams {
            cipher_suites: vec![
                "TLS_AES_128_GCM_SHA256".to_string(),
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
                "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256".to_string(),
                "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256".to_string(),
            ],
            supported_versions: vec!["1.3".to_string(), "1.2".to_string()],
            compression_methods: vec![],
        }
    }

    fn apply_full_obfuscation(&self) -> ObfuscatedTlsParams {
        // 完全随机化 TLS 参数
        let mut base_params = self.apply_advanced_obfuscation();
        
        // 添加额外的加密套件
        let additional_suites = vec![
            "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string(),
            "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384".to_string(),
            "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256".to_string(),
        ];
        
        // 使用 rand crate 随机选择要添加的套件
        let mut rng = rand::rng();
        
        // 随机添加 1-3 个额外套件
        let count = rng.random_range(1..=3);
        for i in 0..count.min(additional_suites.len()) {
            if !base_params.cipher_suites.contains(&additional_suites[i]) {
                base_params.cipher_suites.push(additional_suites[i].clone());
            }
        }
        
        base_params
    }
}

/// 混淆后的 TLS 参数
#[derive(Debug, Clone)]
pub struct ObfuscatedTlsParams {
    /// 加密套件列表
    pub cipher_suites: Vec<String>,
    /// 支持的 TLS 版本
    pub supported_versions: Vec<String>,
    /// 压缩方法
    pub compression_methods: Vec<String>,
}

impl Default for ObfuscatedTlsParams {
    fn default() -> Self {
        Self {
            cipher_suites: vec!["TLS_AES_128_GCM_SHA256".to_string()],
            supported_versions: vec!["1.3".to_string()],
            compression_methods: vec![],
        }
    }
}

/// 生成 Canvas 指纹混淆数据
///
/// 用于对抗基于 Canvas 的浏览器指纹识别
/// 使用 rand crate 生成高质量随机噪声
///
/// # 返回
///
/// 随机生成的 256 字节噪声数据向量
pub fn generate_canvas_noise() -> Vec<u8> {
    let mut rng = rand::rng();
    let mut noise = vec![0u8; 256];
    rng.fill(&mut noise[..]);
    noise
}

/// 生成 WebGL 指纹混淆数据
///
/// 用于对抗基于 WebGL 的浏览器指纹识别
/// 使用 rand crate 从真实渲染器列表中随机选择
///
/// # 返回
///
/// 随机选择的 WebGL 渲染器字符串
pub fn generate_webgl_noise() -> String {
    // 常见的 WebGL 渲染器字符串
    let renderers = vec![
        "ANGLE (Intel, Intel(R) UHD Graphics 620, OpenGL 4.5)",
        "ANGLE (NVIDIA, NVIDIA GeForce GTX 1660 Ti Direct3D11 vs_5_0 ps_5_0, D3D11)",
        "ANGLE (AMD, AMD Radeon RX 580 Series Direct3D11 vs_5_0 ps_5_0, D3D11)",
        "WebKit WebGL",
        "Mozilla - Intel Open Source Technology Center Mesa DRI Intel(R) HD Graphics",
        "ANGLE (Intel, Intel(R) UHD Graphics 630, OpenGL 4.6)",
        "ANGLE (NVIDIA, NVIDIA GeForce RTX 2060 Direct3D11 vs_5_0 ps_5_0, D3D11)",
        "ANGLE (AMD, Radeon RX Vega 8 Graphics Direct3D11 vs_5_0 ps_5_0, D3D11)",
    ];
    
    // 使用 rand crate 随机选择
    let mut rng = rand::rng();
    let index = rng.random_range(0..renderers.len());
    renderers[index].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_protector_new() {
        let protector = FingerprintProtector::new(TlsFingerprintLevel::Basic);
        let params = protector.get_obfuscated_params();
        assert!(!params.cipher_suites.is_empty());
    }

    #[test]
    fn test_fingerprint_protector_basic() {
        let protector = FingerprintProtector::new(TlsFingerprintLevel::Basic);
        let params = protector.get_obfuscated_params();
        assert_eq!(params.cipher_suites.len(), 2);
    }

    #[test]
    fn test_fingerprint_protector_advanced() {
        let protector = FingerprintProtector::new(TlsFingerprintLevel::Advanced);
        let params = protector.get_obfuscated_params();
        assert!(params.cipher_suites.len() >= 5);
    }

    #[test]
    fn test_fingerprint_protector_full() {
        let protector = FingerprintProtector::new(TlsFingerprintLevel::Full);
        let params = protector.get_obfuscated_params();
        assert!(params.cipher_suites.len() > 5);
    }

    #[test]
    fn test_generate_canvas_noise() {
        let noise = generate_canvas_noise();
        assert_eq!(noise.len(), 256);
    }

    #[test]
    fn test_generate_webgl_noise() {
        let noise = generate_webgl_noise();
        assert!(!noise.is_empty());
    }
}
