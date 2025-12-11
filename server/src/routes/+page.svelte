<!-- 首页 -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { SearchBox, HotTrendCard, Loading, ErrorMessage } from '$lib/components';
	import { apiClient, type HotAllResponse } from '$lib/api';
	import { onMount } from 'svelte';

	let loading = $state(true);
	let error = $state<string | null>(null);
	let hotTrends = $state<HotAllResponse | null>(null);
	let searchQuery = $state('');

	async function loadHotTrends() {
		loading = true;
		error = null;
		try {
			hotTrends = await apiClient.getAllHotTrends();
		} catch (e) {
			error = e instanceof Error ? e.message : '加载热门趋势失败';
		} finally {
			loading = false;
		}
	}

	function handleSearch(e: CustomEvent<string>) {
		const query = e.detail;
		goto(`/search?q=${encodeURIComponent(query)}`);
	}

	function handleHotSearch(e: CustomEvent<string>) {
		const query = e.detail;
		goto(`/search?q=${encodeURIComponent(query)}`);
	}

	onMount(() => {
		loadHotTrends();
	});
</script>

<div class="home-page page-enter">
	<!-- Hero 区域 -->
	<section class="hero">
		<div class="hero-content">
			<h1 class="hero-title">
				<span class="gradient-text">SeeSea</span>
				<span class="subtitle">智能搜索引擎</span>
			</h1>
			<p class="hero-description">
				聚合多个搜索引擎，一次搜索，全网结果
			</p>
			<div class="search-wrapper">
				<SearchBox
					bind:value={searchQuery}
					placeholder="输入你想搜索的内容..."
					autofocus={true}
					on:search={handleSearch}
				/>
			</div>
			<div class="quick-tags">
				<span class="tag-label">快速搜索：</span>
				{#each ['Rust 编程', 'AI 新闻', 'GitHub 开源', '技术博客'] as tag}
					<button class="quick-tag" onclick={() => goto(`/search?q=${encodeURIComponent(tag)}`)}>
						{tag}
					</button>
				{/each}
			</div>
		</div>
		<div class="hero-decoration">
			<div class="blob blob-1"></div>
			<div class="blob blob-2"></div>
			<div class="blob blob-3"></div>
		</div>
	</section>

	<!-- 热门趋势区域 -->
	<section class="hot-section">
		<h2 class="section-title">
			<span class="title-icon">🔥</span>
			实时热榜
		</h2>

		{#if loading}
			<Loading text="加载热门趋势中..." />
		{:else if error}
			<ErrorMessage message={error} retry={loadHotTrends} />
		{:else if hotTrends && hotTrends.results.length > 0}
			<div class="hot-grid">
				{#each hotTrends.results.slice(0, 6) as trend}
					<HotTrendCard {trend} maxItems={8} on:search={handleHotSearch} />
				{/each}
			</div>
			{#if hotTrends.results.length > 6}
				<div class="view-all">
					<a href="/hot" class="view-all-btn">
						查看全部热榜
						<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M5 12h14M12 5l7 7-7 7"/>
						</svg>
					</a>
				</div>
			{/if}
		{:else}
			<p class="no-data">暂无热门趋势数据</p>
		{/if}
	</section>

	<!-- 功能介绍 -->
	<section class="features-section">
		<h2 class="section-title">
			<span class="title-icon">✨</span>
			核心功能
		</h2>
		<div class="features-grid">
			<div class="feature-card">
				<div class="feature-icon">🔍</div>
				<h3>多引擎聚合</h3>
				<p>同时从 Google、Bing、DuckDuckGo 等多个引擎获取结果</p>
			</div>
			<div class="feature-card">
				<div class="feature-icon">🚀</div>
				<h3>智能缓存</h3>
				<p>高效缓存机制，重复搜索毫秒级响应</p>
			</div>
			<div class="feature-card">
				<div class="feature-icon">📡</div>
				<h3>RSS 订阅</h3>
				<p>支持 RSS 源管理，实时追踪感兴趣的内容</p>
			</div>
			<div class="feature-card">
				<div class="feature-icon">🔥</div>
				<h3>实时热榜</h3>
				<p>聚合各大平台热门内容，不错过任何热点</p>
			</div>
			<div class="feature-card">
				<div class="feature-icon">🤖</div>
				<h3>Pro 增强搜索</h3>
				<p>基于向量检索的智能排序，结果更精准</p>
			</div>
			<div class="feature-card">
				<div class="feature-icon">📊</div>
				<h3>数据统计</h3>
				<p>实时监控搜索性能和系统指标</p>
			</div>
		</div>
	</section>
</div>

<style>
	.home-page {
		padding-bottom: 4rem;
	}

	/* Hero 区域 */
	.hero {
		position: relative;
		padding: 4rem 0 3rem;
		overflow: hidden;
	}

	.hero-content {
		position: relative;
		z-index: 1;
		text-align: center;
	}

	.hero-title {
		margin: 0 0 1rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
	}

	.gradient-text {
		font-size: 3.5rem;
		font-weight: 800;
		background: linear-gradient(135deg, #3b82f6 0%, #06b6d4 50%, #10b981 100%);
		-webkit-background-clip: text;
		background-clip: text;
		-webkit-text-fill-color: transparent;
	}

	.subtitle {
		font-size: 1.25rem;
		font-weight: 500;
		color: #6b7280;
	}

	.hero-description {
		font-size: 1.125rem;
		color: #6b7280;
		margin: 0 0 2rem;
	}

	.search-wrapper {
		display: flex;
		justify-content: center;
		margin-bottom: 1.5rem;
	}

	.quick-tags {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.tag-label {
		font-size: 0.875rem;
		color: #9ca3af;
	}

	.quick-tag {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		color: #6b7280;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 9999px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.quick-tag:hover {
		color: #3b82f6;
		border-color: #3b82f6;
		background: #eff6ff;
	}

	/* 背景装饰 */
	.hero-decoration {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
		z-index: 0;
	}

	.blob {
		position: absolute;
		border-radius: 50%;
		filter: blur(60px);
		opacity: 0.3;
		will-change: transform;
		transform: translateZ(0);
		backface-visibility: hidden;
	}

	.blob-1 {
		width: 400px;
		height: 400px;
		background: linear-gradient(135deg, #3b82f6, #06b6d4);
		top: -100px;
		right: -100px;
	}

	.blob-2 {
		width: 300px;
		height: 300px;
		background: linear-gradient(135deg, #10b981, #3b82f6);
		bottom: -50px;
		left: -50px;
	}

	.blob-3 {
		width: 200px;
		height: 200px;
		background: linear-gradient(135deg, #f59e0b, #ef4444);
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%) translateZ(0);
	}

	/* 热门趋势区域 */
	.hot-section {
		padding: 2rem 0;
	}

	.section-title {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0 0 1.5rem;
		font-size: 1.5rem;
		font-weight: 700;
		color: #1f2937;
	}

	.title-icon {
		font-size: 1.25rem;
	}

	.hot-grid {
		display: grid;
		gap: 1.5rem;
		grid-template-columns: repeat(1, 1fr);
	}

	@media (min-width: 640px) {
		.hot-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	@media (min-width: 1024px) {
		.hot-grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.view-all {
		display: flex;
		justify-content: center;
		margin-top: 2rem;
	}

	.view-all-btn {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1.5rem;
		font-size: 0.9375rem;
		font-weight: 500;
		color: #3b82f6;
		background: #eff6ff;
		border-radius: 8px;
		text-decoration: none;
		transition: all 0.2s;
	}

	.view-all-btn:hover {
		background: #dbeafe;
		transform: translateX(4px);
	}

	.no-data {
		text-align: center;
		color: #9ca3af;
		padding: 3rem 0;
	}

	/* 功能介绍 */
	.features-section {
		padding: 2rem 0;
	}

	.features-grid {
		display: grid;
		gap: 1.5rem;
		grid-template-columns: repeat(1, 1fr);
	}

	@media (min-width: 640px) {
		.features-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	@media (min-width: 1024px) {
		.features-grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.feature-card {
		background: white;
		border-radius: 16px;
		padding: 1.5rem;
		border: 1px solid #e5e7eb;
		transition: all 0.3s ease;
	}

	.feature-card:hover {
		border-color: #d1d5db;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.06);
		transform: translateY(-4px);
	}

	.feature-icon {
		font-size: 2rem;
		margin-bottom: 0.75rem;
	}

	.feature-card h3 {
		margin: 0 0 0.5rem;
		font-size: 1.125rem;
		font-weight: 600;
		color: #1f2937;
	}

	.feature-card p {
		margin: 0;
		font-size: 0.9375rem;
		color: #6b7280;
		line-height: 1.6;
	}

	/* 移动端适配 */
	@media (max-width: 640px) {
		.home-page {
			padding-bottom: 3rem;
		}

		.hero {
			padding: 2rem 0 1.5rem;
		}

		.hero-title {
			margin-bottom: 0.75rem;
			gap: 0.375rem;
		}

		.gradient-text {
			font-size: 2.5rem;
		}

		.subtitle {
			font-size: 1rem;
		}

		.hero-description {
			font-size: 0.9375rem;
			margin-bottom: 1.5rem;
		}

		.search-wrapper {
			margin-bottom: 1rem;
		}

		.quick-tags {
			gap: 0.375rem;
		}

		.tag-label {
			font-size: 0.8125rem;
			width: 100%;
			text-align: center;
			margin-bottom: 0.25rem;
		}

		.quick-tag {
			padding: 0.3125rem 0.625rem;
			font-size: 0.75rem;
		}

		.blob-1 {
			width: 250px;
			height: 250px;
			top: -80px;
			right: -80px;
		}

		.blob-2 {
			width: 200px;
			height: 200px;
			bottom: -80px;
			left: -80px;
		}

		.blob-3 {
			width: 120px;
			height: 120px;
		}

		.hot-section,
		.features-section {
			padding: 1.5rem 0;
		}

		.section-title {
			font-size: 1.25rem;
			margin-bottom: 1rem;
			gap: 0.375rem;
		}

		.title-icon {
			font-size: 1.125rem;
		}

		.hot-grid,
		.features-grid {
			gap: 0.875rem;
		}

		.view-all {
			margin-top: 1.5rem;
		}

		.view-all-btn {
			padding: 0.625rem 1.25rem;
			font-size: 0.875rem;
		}

		.view-all-btn:hover {
			transform: none;
		}

		.feature-card {
			padding: 1.25rem;
			border-radius: 12px;
		}

		.feature-card:hover {
			transform: none;
		}

		.feature-icon {
			font-size: 1.75rem;
			margin-bottom: 0.5rem;
		}

		.feature-card h3 {
			font-size: 1rem;
			margin-bottom: 0.375rem;
		}

		.feature-card p {
			font-size: 0.875rem;
		}

		.no-data {
			padding: 2rem 0;
			font-size: 0.875rem;
		}
	}

	/* 超小屏适配 */
	@media (max-width: 380px) {
		.hero {
			padding: 1.5rem 0 1rem;
		}

		.gradient-text {
			font-size: 2rem;
		}

		.subtitle {
			font-size: 0.875rem;
		}

		.hero-description {
			font-size: 0.875rem;
		}

		.quick-tags {
			gap: 0.25rem;
		}

		.quick-tag {
			padding: 0.25rem 0.5rem;
			font-size: 0.6875rem;
		}

		.section-title {
			font-size: 1.125rem;
		}

		.hot-grid,
		.features-grid {
			gap: 0.75rem;
		}

		.feature-card {
			padding: 1rem;
		}

		.feature-icon {
			font-size: 1.5rem;
		}

		.feature-card h3 {
			font-size: 0.9375rem;
		}

		.feature-card p {
			font-size: 0.8125rem;
		}
	}
</style>

