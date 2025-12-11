<!-- 统计页面 -->
<script lang="ts">
	import { StatCard, Loading, ErrorMessage } from '$lib/components';
	import { apiClient, type StatsResponse, type RealtimeMetrics, type CacheStats } from '$lib/api';
	import { onMount, onDestroy } from 'svelte';

	let loading = $state(true);
	let error = $state<string | null>(null);
	let stats = $state<StatsResponse | null>(null);
	let metrics = $state<RealtimeMetrics | null>(null);
	let cacheStats = $state<CacheStats | null>(null);
	let refreshInterval: ReturnType<typeof setInterval> | null = null;
	let lastUpdate = $state<Date | null>(null);

	async function loadData() {
		error = null;
		try {
			const [statsData, metricsData, cacheData] = await Promise.all([
				apiClient.getStats(),
				apiClient.getRealtimeMetrics(),
				apiClient.getCacheStats()
			]);
			stats = statsData;
			metrics = metricsData;
			cacheStats = cacheData;
			lastUpdate = new Date();
		} catch (e) {
			error = e instanceof Error ? e.message : '加载统计数据失败';
		} finally {
			loading = false;
		}
	}

	function formatUptime(seconds: number): string {
		if (!seconds || isNaN(seconds)) return '0 分钟';
		const days = Math.floor(seconds / 86400);
		const hours = Math.floor((seconds % 86400) / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);

		if (days > 0) return `${days} 天 ${hours} 小时`;
		if (hours > 0) return `${hours} 小时 ${minutes} 分钟`;
		return `${minutes} 分钟`;
	}

	function formatBytes(bytes: number): string {
		if (!bytes || bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
	}

	function formatTime(date: Date | null): string {
		if (!date) return '';
		return date.toLocaleTimeString('zh-CN');
	}

	function formatNumber(num: number | undefined): string {
		if (num === undefined || num === null || isNaN(num)) return '0';
		return num.toLocaleString();
	}

	function formatPercent(num: number | undefined): string {
		if (num === undefined || num === null || isNaN(num)) return '0.0';
		return (num * 100).toFixed(1);
	}

	function formatMs(num: number | undefined): string {
		if (num === undefined || num === null || isNaN(num)) return '0.0';
		return num.toFixed(1);
	}

	onMount(() => {
		loadData();
		// 每30秒自动刷新
		refreshInterval = setInterval(loadData, 30000);
	});

	onDestroy(() => {
		if (refreshInterval) {
			clearInterval(refreshInterval);
		}
	});
</script>

<svelte:head>
	<title>统计 - SeeSea</title>
</svelte:head>

<div class="stats-page page-enter">
	<header class="page-header">
		<div class="header-content">
			<h1 class="page-title">
				<span class="title-icon">📊</span>
				系统统计
			</h1>
			<p class="page-description">实时监控搜索性能和系统指标</p>
		</div>
		<div class="header-actions">
			<button class="refresh-btn" onclick={loadData} disabled={loading}>
				<svg class="refresh-icon" class:spinning={loading} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M1 4v6h6M23 20v-6h-6" />
					<path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15" />
				</svg>
				刷新
			</button>
			{#if lastUpdate}
				<span class="last-update">上次更新: {formatTime(lastUpdate)}</span>
			{/if}
		</div>
	</header>

	{#if loading && !stats}
		<Loading text="加载统计数据中..." size="lg" />
	{:else if error}
		<ErrorMessage message={error} retry={loadData} />
	{:else}
		<!-- 核心指标 -->
		<section class="section">
			<h2 class="section-title">核心指标</h2>
			<div class="stats-grid">
				{#if stats}
					<StatCard
						label="总搜索次数"
						value={formatNumber(stats.total_searches)}
						icon="🔍"
						color="blue"
					/>
					<StatCard
						label="缓存命中"
						value={formatNumber(stats.cache_hits)}
						icon="✅"
						color="green"
					/>
					<StatCard
						label="缓存未命中"
						value={formatNumber(stats.cache_misses)}
						icon="❌"
						color="orange"
					/>
					<StatCard
						label="缓存命中率"
						value={formatPercent(stats.cache_hit_rate)}
						suffix="%"
						icon="🎯"
						color="purple"
					/>
					<StatCard
						label="引擎失败"
						value={formatNumber(stats.engine_failures)}
						icon="⚠️"
						color="red"
					/>
					<StatCard
						label="超时次数"
						value={formatNumber(stats.timeouts)}
						icon="⏱️"
						color="orange"
					/>
				{/if}
			</div>
		</section>

		<!-- 实时指标 -->
		{#if metrics}
			<section class="section">
				<h2 class="section-title">实时指标</h2>
				<div class="stats-grid">
					<StatCard
						label="运行时间"
						value={formatUptime(metrics.uptime_seconds)}
						icon="⏱️"
						color="blue"
					/>
					<StatCard
						label="请求总数"
						value={formatNumber(metrics.total_requests)}
						icon="📈"
						color="blue"
					/>
					<StatCard
						label="成功请求"
						value={formatNumber(metrics.successful_requests)}
						icon="✅"
						color="green"
					/>
					<StatCard
						label="失败请求"
						value={formatNumber(metrics.failed_requests)}
						icon="⚠️"
						color="red"
					/>
					<StatCard
						label="平均响应时间"
						value={formatMs(metrics.avg_response_time_ms)}
						suffix=" ms"
						icon="⚡"
						color="purple"
					/>
					<StatCard
						label="速率限制"
						value={formatNumber(metrics.rate_limited)}
						icon="🚫"
						color="orange"
					/>
					<StatCard
						label="熔断触发"
						value={formatNumber(metrics.circuit_breaker_trips)}
						icon="🔌"
						color="red"
					/>
					<StatCard
						label="IP 封禁"
						value={formatNumber(metrics.ip_blocked)}
						icon="🛡️"
						color="orange"
					/>
					<StatCard
						label="活跃连接"
						value={formatNumber(metrics.active_connections)}
						icon="🔗"
						color="blue"
					/>
				</div>
			</section>
		{/if}

		<!-- 缓存统计 -->
		{#if cacheStats}
			<section class="section">
				<h2 class="section-title">
					缓存统计
					{#if cacheStats.total_entries === 0 && cacheStats.size_bytes === 0}
						<span class="section-hint">（数据收集中...）</span>
					{/if}
				</h2>
				<div class="stats-grid">
					<StatCard
						label="总缓存条目"
						value={formatNumber(cacheStats.total_entries)}
						icon="📦"
						color="blue"
					/>
					<StatCard
						label="缓存大小"
						value={formatBytes(cacheStats.size_bytes)}
						icon="💾"
						color="purple"
					/>
					<StatCard
						label="命中率"
						value={formatPercent(cacheStats.hit_rate)}
						suffix="%"
						icon="🎯"
						color="green"
					/>
					<StatCard
						label="缓存命中"
						value={formatNumber(cacheStats.hits)}
						icon="✅"
						color="green"
					/>
					<StatCard
						label="缓存未命中"
						value={formatNumber(cacheStats.misses)}
						icon="❌"
						color="orange"
					/>
					<StatCard
						label="写入次数"
						value={formatNumber(cacheStats.writes)}
						icon="✏️"
						color="blue"
					/>
					<StatCard
						label="删除次数"
						value={formatNumber(cacheStats.deletes)}
						icon="�️"
						color="red"
					/>
					<StatCard
						label="过期清理"
						value={formatNumber(cacheStats.evictions)}
						icon="🧹"
						color="orange"
					/>
					<StatCard
						label="读取延迟"
						value={formatMs(cacheStats.avg_get_latency_ms)}
						suffix=" ms"
						icon="⚡"
						color="purple"
					/>
					<StatCard
						label="写入延迟"
						value={formatMs(cacheStats.avg_set_latency_ms)}
						suffix=" ms"
						icon="⏱️"
						color="purple"
					/>
				</div>
			</section>
		{/if}

		<!-- 系统状态 -->
		<section class="section">
			<h2 class="section-title">系统状态</h2>
			<div class="status-card">
				<div class="status-item">
					<span class="status-indicator online"></span>
					<span class="status-label">API 服务</span>
					<span class="status-value">运行中</span>
				</div>
				<div class="status-item">
					<span class="status-indicator online"></span>
					<span class="status-label">缓存服务</span>
					<span class="status-value">运行中</span>
				</div>
				<div class="status-item">
					<span class="status-indicator" class:online={!metrics || metrics.circuit_breaker_trips === 0} class:warning={metrics && metrics.circuit_breaker_trips > 0}></span>
					<span class="status-label">熔断器</span>
					<span class="status-value">{metrics && metrics.circuit_breaker_trips > 0 ? '已触发' : '正常'}</span>
				</div>
			</div>
		</section>
	{/if}
</div>

<style>
	.stats-page {
		padding: 2rem 0 4rem;
	}

	.page-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		margin-bottom: 2rem;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.header-content {
		flex: 1;
	}

	.page-title {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0 0 0.5rem;
		font-size: 2rem;
		font-weight: 700;
		color: #1f2937;
	}

	.title-icon {
		font-size: 1.75rem;
	}

	.page-description {
		margin: 0;
		font-size: 1rem;
		color: #6b7280;
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.refresh-btn {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #6b7280;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.refresh-btn:hover:not(:disabled) {
		border-color: #3b82f6;
		color: #3b82f6;
	}

	.refresh-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.refresh-icon {
		transition: transform 0.3s;
	}

	.refresh-icon.spinning {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	.last-update {
		font-size: 0.75rem;
		color: #9ca3af;
	}

	.section {
		margin-bottom: 2rem;
	}

	.section-title {
		margin: 0 0 1rem;
		font-size: 1.25rem;
		font-weight: 600;
		color: #1f2937;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.section-hint {
		font-size: 0.75rem;
		font-weight: 400;
		color: #9ca3af;
	}

	.stats-grid {
		display: grid;
		gap: 1rem;
		grid-template-columns: repeat(2, 1fr);
	}

	@media (min-width: 640px) {
		.stats-grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	@media (min-width: 1024px) {
		.stats-grid {
			grid-template-columns: repeat(4, 1fr);
		}
	}

	.status-card {
		background: white;
		border-radius: 12px;
		padding: 1rem;
		border: 1px solid #e5e7eb;
	}

	.status-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 0;
		border-bottom: 1px solid #f3f4f6;
	}

	.status-item:last-child {
		border-bottom: none;
	}

	.status-indicator {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: #d1d5db;
	}

	.status-indicator.online {
		background: #22c55e;
		box-shadow: 0 0 8px rgba(34, 197, 94, 0.4);
	}

	.status-indicator.warning {
		background: #f59e0b;
		box-shadow: 0 0 8px rgba(245, 158, 11, 0.4);
	}

	.status-label {
		flex: 1;
		font-size: 0.9375rem;
		color: #374151;
	}

	.status-value {
		font-size: 0.875rem;
		color: #6b7280;
	}

	/* 移动端适配 */
	@media (max-width: 640px) {
		.stats-page {
			padding: 1rem 0 3rem;
		}

		.page-header {
			flex-direction: column;
			margin-bottom: 1.5rem;
		}

		.page-title {
			font-size: 1.5rem;
			gap: 0.375rem;
		}

		.title-icon {
			font-size: 1.375rem;
		}

		.page-description {
			font-size: 0.875rem;
		}

		.header-actions {
			width: 100%;
			justify-content: space-between;
		}

		.refresh-btn {
			padding: 0.4375rem 0.875rem;
			font-size: 0.8125rem;
		}

		.section {
			margin-bottom: 1.5rem;
		}

		.section-title {
			font-size: 1.0625rem;
			margin-bottom: 0.75rem;
		}

		.stats-grid {
			grid-template-columns: repeat(2, 1fr);
			gap: 0.75rem;
		}

		.status-card {
			padding: 0.75rem;
			border-radius: 10px;
		}

		.status-item {
			padding: 0.625rem 0;
			gap: 0.5rem;
		}

		.status-label {
			font-size: 0.875rem;
		}

		.status-value {
			font-size: 0.8125rem;
		}
	}

	/* 超小屏适配 */
	@media (max-width: 380px) {
		.stats-page {
			padding: 0.75rem 0 2.5rem;
		}

		.page-title {
			font-size: 1.25rem;
		}

		.stats-grid {
			gap: 0.5rem;
		}

		.status-indicator {
			width: 8px;
			height: 8px;
		}

		.status-label {
			font-size: 0.8125rem;
		}

		.status-value {
			font-size: 0.75rem;
		}
	}
</style>
