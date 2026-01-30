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

//! 资源监控器模块
//!
//! 负责实时监控系统资源使用情况，包括CPU、内存、磁盘I/O和网络I/O等

use super::types::ResourceStatus;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Disks, Networks, System};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, warn};

/// 资源监控器
///
/// 实时监控系统资源使用情况，并提供获取资源状态的接口
pub struct ResourceMonitor {
    /// 当前资源状态
    current_status: Arc<RwLock<ResourceStatus>>,
    /// 监控间隔
    monitoring_interval: Duration,
    /// 系统信息对象
    system: Arc<RwLock<System>>,
    /// 磁盘信息对象
    disks: Arc<RwLock<Disks>>,
    /// 网络信息对象
    networks: Arc<RwLock<Networks>>,
    /// 上次磁盘 I/O 统计数据（用于计算 I/O 使用率）
    last_disk_stats: Arc<RwLock<DiskStats>>,
    /// 上次网络 I/O 统计数据（用于计算网络使用率）
    last_net_stats: Arc<RwLock<NetStats>>,
    /// 上次更新时间戳
    last_update_time: Arc<RwLock<std::time::Instant>>,
}

/// 磁盘 I/O 统计数据
#[derive(Debug, Clone, Default)]
struct DiskStats {
    /// 读取字节数
    read_bytes: u64,
    /// 写入字节数
    write_bytes: u64,
    /// 读取次数
    reads: u64,
    /// 写入次数
    writes: u64,
}

/// 网络 I/O 统计数据
#[derive(Debug, Clone, Default)]
struct NetStats {
    /// 接收字节数
    rx_bytes: u64,
    /// 发送字节数
    tx_bytes: u64,
    /// 接收包数
    rx_packets: u64,
    /// 发送包数
    tx_packets: u64,
}

impl Clone for ResourceMonitor {
    fn clone(&self) -> Self {
        Self {
            current_status: self.current_status.clone(),
            monitoring_interval: self.monitoring_interval,
            system: self.system.clone(),
            disks: self.disks.clone(),
            networks: self.networks.clone(),
            last_disk_stats: self.last_disk_stats.clone(),
            last_net_stats: self.last_net_stats.clone(),
            last_update_time: self.last_update_time.clone(),
        }
    }
}

impl ResourceMonitor {
    /// 创建新的资源监控器
    pub fn new(monitoring_interval: Duration) -> Self {
        // 创建 sysinfo System 对象
        let mut system = System::new_all();
        system.refresh_all();

        // 创建 Disks 和 Networks 对象
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        Self {
            current_status: Arc::new(RwLock::new(ResourceStatus::default())),
            monitoring_interval,
            system: Arc::new(RwLock::new(system)),
            disks: Arc::new(RwLock::new(disks)),
            networks: Arc::new(RwLock::new(networks)),
            last_disk_stats: Arc::new(RwLock::new(DiskStats::default())),
            last_net_stats: Arc::new(RwLock::new(NetStats::default())),
            last_update_time: Arc::new(RwLock::new(std::time::Instant::now())),
        }
    }

    /// 获取当前资源状态
    pub async fn get_current_status(&self) -> ResourceStatus {
        self.current_status.read().await.clone()
    }

    /// 启动资源监控
    pub async fn start(&self) {
        let mut interval = interval(self.monitoring_interval);
        debug!(
            "Starting resource monitor with interval: {:?}",
            self.monitoring_interval
        );

        loop {
            interval.tick().await;
            match self.update_resource_status().await {
                Ok(_) => {
                    let status = self.get_current_status().await;
                    debug!(
                        "Resource status updated: CPU={:.2}%, Memory={:.2}%, DiskIO={:.2}%, NetworkIO={:.2}%\n",
                        status.cpu_usage * 100.0,
                        status.memory_usage * 100.0,
                        status.disk_io_usage * 100.0,
                        status.network_io_usage * 100.0
                    );
                }
                Err(e) => {
                    warn!("Failed to update resource status: {}", e);
                }
            }
        }
    }

    /// 更新资源状态
    async fn update_resource_status(&self) -> Result<(), String> {
        // 使用 sysinfo 获取系统信息
        {
            let mut system = self.system.write().await;
            system.refresh_all();
        }

        // 刷新磁盘和网络信息
        {
            let mut disks = self.disks.write().await;
            disks.refresh(false);
        }
        {
            let mut networks = self.networks.write().await;
            networks.refresh(false);
        }

        let system = self.system.read().await;

        // CPU 使用率
        let cpu_usage = system.global_cpu_usage() as f64 / 100.0;

        // 内存信息
        let total_memory = system.total_memory();
        let available_memory = system.available_memory();
        let memory_usage = if total_memory > 0 {
            (total_memory - available_memory) as f64 / total_memory as f64
        } else {
            0.0
        };

        // 负载平均值
        let load_avg = System::load_average();
        let load_avg_1 = load_avg.one;
        let load_avg_5 = load_avg.five;
        let load_avg_15 = load_avg.fifteen;

        // 磁盘信息 - 使用 sysinfo 0.38 的正确 API
        let mut total_disk = 0u64;
        let mut available_disk = 0u64;

        let disks = self.disks.read().await;
        for disk in disks.list() {
            total_disk += disk.total_space();
            available_disk += disk.available_space();
        }

        let disk_usage_percent = if total_disk > 0 {
            (total_disk - available_disk) as f64 / total_disk as f64
        } else {
            0.0
        };

        // 计算磁盘 I/O 使用率
        let disk_io_usage = self.calculate_disk_io_usage().await?;

        // 计算网络 I/O 使用率
        let network_io_usage = self.calculate_network_io_usage().await?;

        // 更新上次更新时间
        *self.last_update_time.write().await = std::time::Instant::now();

        let mut status = self.current_status.write().await;

        // 更新资源状态
        *status = ResourceStatus {
            cpu_usage,
            memory_usage,
            disk_io_usage,
            network_io_usage,
            available_memory,
            available_disk,
            total_disk,
            load_avg_1,
            load_avg_5,
            load_avg_15,
            disk_usage_percent,
        };

        Ok(())
    }

    /// 计算磁盘 I/O 使用率
    ///
    /// 使用平台特定的 API 获取磁盘 I/O 统计数据
    async fn calculate_disk_io_usage(&self) -> Result<f64, String> {
        let mut current_stats = DiskStats::default();

        #[cfg(target_os = "linux")]
        {
            // Linux: 从 /proc/diskstats 读取完整的 I/O 统计
            if let Ok(diskstats) = std::fs::read_to_string("/proc/diskstats") {
                for line in diskstats.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 14 {
                        // 跳过分区，只处理物理设备（设备名不包含数字）
                        let device_name = parts.get(2).unwrap_or(&"");
                        if !device_name.contains(|c: char| c.is_numeric()) {
                            if let Ok(reads) = parts[3].parse::<u64>() {
                                current_stats.reads += reads;
                            }
                            if let Ok(writes) = parts[7].parse::<u64>() {
                                current_stats.writes += writes;
                            }
                            // 读取扇区数转换为字节数（1 sector = 512 bytes）
                            if let Ok(read_sectors) = parts[5].parse::<u64>() {
                                current_stats.read_bytes += read_sectors * 512;
                            }
                            if let Ok(write_sectors) = parts[9].parse::<u64>() {
                                current_stats.write_bytes += write_sectors * 512;
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Windows: 使用 GetDiskPerformanceStatistics API
            use windows::Win32::Foundation::{GENERIC_READ, INVALID_HANDLE_VALUE};
            use windows::Win32::Storage::FileSystem::{
                CreateFileW, FILE_CREATION_DISPOSITION, FILE_FLAG_BACKUP_SEMANTICS,
                FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE, GetDriveTypeW, GetLogicalDriveStringsW,
                OPEN_EXISTING,
            };
            use windows::Win32::System::IO::DeviceIoControl;
            use windows::Win32::System::Ioctl::{DISK_PERFORMANCE, IOCTL_DISK_PERFORMANCE};
            use windows::Win32::System::WindowsProgramming::DRIVE_FIXED;
            use windows::core::PCWSTR;

            let mut drives = [0u16; 256];
            unsafe {
                let len = GetLogicalDriveStringsW(Some(&mut drives));
                if len > 0 {
                    let mut i = 0;
                    while i < drives.len() {
                        if drives[i] == 0 {
                            break;
                        }
                        let drive_str = &drives[i..i + 4];
                        if GetDriveTypeW(PCWSTR(drive_str.as_ptr())) == DRIVE_FIXED {
                            // 构造设备路径
                            let device_path = format!(
                                "\\\\.\\{}:",
                                std::char::from_u32(drives[i] as u32).unwrap()
                            );
                            let device_path_wide: Vec<u16> = device_path
                                .encode_utf16()
                                .chain(std::iter::once(0))
                                .collect();

                            // 打开设备句柄
                            let handle = CreateFileW(
                                PCWSTR(device_path_wide.as_ptr()),
                                GENERIC_READ.0,
                                FILE_SHARE_MODE(3), // FILE_SHARE_READ | FILE_SHARE_WRITE
                                None,
                                FILE_CREATION_DISPOSITION(OPEN_EXISTING.0),
                                FILE_FLAGS_AND_ATTRIBUTES(FILE_FLAG_BACKUP_SEMANTICS.0),
                                None,
                            );

                            if handle.is_ok() && handle.as_ref().unwrap() != &INVALID_HANDLE_VALUE {
                                let handle = handle.unwrap();
                                // 获取磁盘性能统计
                                let mut perf_stats = DISK_PERFORMANCE::default();
                                let mut bytes_returned = 0u32;

                                let result = DeviceIoControl(
                                    handle,
                                    IOCTL_DISK_PERFORMANCE,
                                    None,
                                    0,
                                    Some(&mut perf_stats as *mut _ as *mut core::ffi::c_void),
                                    std::mem::size_of::<DISK_PERFORMANCE>() as u32,
                                    Some(&mut bytes_returned),
                                    None,
                                );

                                if result.is_ok() && bytes_returned > 0 {
                                    current_stats.read_bytes += perf_stats.BytesRead as u64;
                                    current_stats.write_bytes += perf_stats.BytesWritten as u64;
                                    current_stats.reads += perf_stats.ReadCount as u64;
                                    current_stats.writes += perf_stats.WriteCount as u64;
                                }
                            }
                        }
                        i += 4;
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            // macOS: 使用 sysctl 获取磁盘 I/O 统计
            use libc::{CTL_HW, HW_DISKSTATS, c_void, sysctl};

            unsafe {
                // macOS 使用 diskstats 结构体
                #[repr(C)]
                #[derive(Clone)]
                struct DiskStats {
                    dk_name: [libc::c_char; 128],
                    dk_bs: libc::c_uint,
                    dk_rbytes: libc::c_longlong,
                    dk_wbytes: libc::c_longlong,
                    dk_rxfer: libc::c_uint,
                    dk_wxfer: libc::c_uint,
                    dk_time: libc::c_uint,
                }

                // 获取磁盘数量
                // 注意: libc::DISKCOUNT 在某些版本中可能不存在
                // 使用替代方法：直接请求所有磁盘统计信息，然后计算数量
                let num_disks: libc::c_int = 32; // 假设最多 32 个磁盘

                // 获取所有磁盘的统计信息
                let mut disk_stats: Vec<DiskStats> = vec![std::mem::zeroed(); num_disks as usize];
                let mut size = std::mem::size_of::<DiskStats>() * num_disks as usize;

                let mut mib = [CTL_HW, HW_DISKSTATS];
                if sysctl(
                    mib.as_mut_ptr(),
                    2,
                    disk_stats.as_mut_ptr() as *mut c_void,
                    &mut size,
                    std::ptr::null_mut(),
                    0,
                ) == 0
                {
                    let actual_num_disks = size / std::mem::size_of::<DiskStats>();

                    // 累加所有磁盘的统计信息
                    for disk in disk_stats.iter().take(actual_num_disks) {
                        current_stats.read_bytes += disk.dk_rbytes as u64;
                        current_stats.write_bytes += disk.dk_wbytes as u64;
                        current_stats.reads += disk.dk_rxfer as u64;
                        current_stats.writes += disk.dk_wxfer as u64;
                    }
                }

                // 如果上面的方法失败，尝试使用 IOKit（需要额外的依赖）
                if current_stats.reads == 0 && current_stats.writes == 0 {
                    // 使用 iostat 命令作为后备方案
                    if let Ok(output) = std::process::Command::new("iostat")
                        .arg("-d")
                        .arg("-I")
                        .output()
                    {
                        if output.status.success() {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            // 解析 iostat 输出
                            for line in stdout.lines().skip(2) {
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if parts.len() >= 6 {
                                    // iostat 格式: disk KB/t tps MB/s KB/s tps MB/s
                                    if let Ok(kb_read) = parts[2].parse::<f64>() {
                                        current_stats.read_bytes += (kb_read * 1024.0) as u64;
                                    }
                                    if let Ok(kb_write) = parts[3].parse::<f64>() {
                                        current_stats.write_bytes += (kb_write * 1024.0) as u64;
                                    }
                                    if let Ok(read_ops) = parts[1].parse::<f64>() {
                                        current_stats.reads += read_ops as u64;
                                    }
                                    if let Ok(write_ops) = parts[4].parse::<f64>() {
                                        current_stats.writes += write_ops as u64;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 获取上次统计数据
        let last_stats = self.last_disk_stats.read().await.clone();
        let last_update = *self.last_update_time.read().await;
        let elapsed_ms = last_update.elapsed().as_millis() as u64;

        // 计算变化量
        let delta_read_bytes = current_stats
            .read_bytes
            .saturating_sub(last_stats.read_bytes);
        let delta_write_bytes = current_stats
            .write_bytes
            .saturating_sub(last_stats.write_bytes);
        let total_delta_bytes = delta_read_bytes + delta_write_bytes;
        let delta_reads = current_stats.reads.saturating_sub(last_stats.reads);
        let delta_writes = current_stats.writes.saturating_sub(last_stats.writes);

        // 更新上次统计数据
        *self.last_disk_stats.write().await = current_stats;

        // 如果是第一次更新或时间间隔太短，返回 0
        if elapsed_ms == 0 {
            return Ok(0.0);
        }

        // 计算 I/O 使用率
        // 方法1: 基于数据传输量（假设磁盘带宽为 100 MB/s）
        let max_bytes_per_ms = 100 * 1024 * 1024 / 1000; // 100 MB/s 转换为 bytes/ms
        let max_bytes = max_bytes_per_ms * elapsed_ms;
        let io_usage_by_bytes = if max_bytes > 0 {
            (total_delta_bytes as f64 / max_bytes as f64).min(1.0)
        } else {
            0.0
        };

        // 方法2: 基于操作频率（假设磁盘支持 1000 IOPS）
        let total_ops = delta_reads + delta_writes;
        let max_ops_per_ms = 1; // 1000 IOPS 转换为 ops/ms
        let max_ops = max_ops_per_ms * elapsed_ms;
        let io_usage_by_ops = if max_ops > 0 {
            (total_ops as f64 / max_ops as f64).min(1.0)
        } else {
            0.0
        };

        // 取两种方法的最大值作为综合使用率
        let io_usage = io_usage_by_bytes.max(io_usage_by_ops);

        Ok(io_usage)
    }
    /// 计算网络 I/O 使用率
    ///
    /// 使用 sysinfo 0.38 获取网络 I/O 统计数据（跨平台完整实现）
    async fn calculate_network_io_usage(&self) -> Result<f64, String> {
        let mut current_stats = NetStats::default();

        // 使用 sysinfo 获取网络统计（跨平台支持）
        let networks = self.networks.read().await;
        for (_name, data) in networks.iter() {
            // 跳过回环设备（需要通过其他方式过滤）
            current_stats.rx_bytes += data.total_received();
            current_stats.tx_bytes += data.total_transmitted();
            current_stats.rx_packets += data.total_packets_received();
            current_stats.tx_packets += data.total_packets_transmitted();
        }

        // 获取上次统计数据
        let last_stats = self.last_net_stats.read().await.clone();
        let last_update = *self.last_update_time.read().await;
        let elapsed_ms = last_update.elapsed().as_millis() as u64;

        // 计算变化量
        let delta_rx_bytes = current_stats.rx_bytes.saturating_sub(last_stats.rx_bytes);
        let delta_tx_bytes = current_stats.tx_bytes.saturating_sub(last_stats.tx_bytes);
        let total_delta_bytes = delta_rx_bytes + delta_tx_bytes;
        let delta_rx_packets = current_stats
            .rx_packets
            .saturating_sub(last_stats.rx_packets);
        let delta_tx_packets = current_stats
            .tx_packets
            .saturating_sub(last_stats.tx_packets);
        let total_delta_packets = delta_rx_packets + delta_tx_packets;

        // 更新上次统计数据
        *self.last_net_stats.write().await = current_stats;

        // 如果是第一次更新或时间间隔太短，返回 0
        if elapsed_ms == 0 {
            return Ok(0.0);
        }

        // 计算网络使用率
        // 方法1: 基于字节数（假设网络带宽为 1 Gbps = 125 MB/s）
        let max_bytes_per_ms = 125 * 1024 * 1024 / 1000; // 125 MB/s 转换为 bytes/ms
        let max_bytes = max_bytes_per_ms * elapsed_ms;
        let network_usage_by_bytes = if max_bytes > 0 {
            (total_delta_bytes as f64 / max_bytes as f64).min(1.0)
        } else {
            0.0
        };

        // 方法2: 基于包数（假设网络支持 1M PPS）
        let max_packets_per_ms = 1_000_000 / 1000; // 1M PPS 转换为 packets/ms
        let max_packets = max_packets_per_ms * elapsed_ms;
        let network_usage_by_packets = if max_packets > 0 {
            (total_delta_packets as f64 / max_packets as f64).min(1.0)
        } else {
            0.0
        };

        // 取两种方法的最大值作为综合使用率
        let network_usage = network_usage_by_bytes.max(network_usage_by_packets);

        Ok(network_usage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_resource_monitor() {
        let monitor = ResourceMonitor::new(Duration::from_millis(100));

        // 启动监控
        let monitor_clone = monitor.clone();
        tokio::spawn(async move {
            monitor_clone.start().await;
        });

        // 等待一段时间，让监控器有时间更新资源状态
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 获取当前资源状态
        let status = monitor.get_current_status().await;

        // 验证资源状态的合理性
        assert!(status.cpu_usage >= 0.0 && status.cpu_usage <= 1.0);
        assert!(status.memory_usage >= 0.0 && status.memory_usage <= 1.0);
        assert!(status.disk_io_usage >= 0.0 && status.disk_io_usage <= 1.0);
        assert!(status.network_io_usage >= 0.0 && status.network_io_usage <= 1.0);
        assert!(status.available_memory > 0);
        assert!(status.available_disk > 0);
    }
}
