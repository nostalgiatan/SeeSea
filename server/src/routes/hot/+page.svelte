<!-- 热榜页面 -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { HotTrendCard, Loading, ErrorMessage, EmptyState } from '$lib/components';
	import { apiClient, type HotAllResponse, type Platform } from '$lib/api';
	import { onMount } from 'svelte';

	let loading = $state(true);
	let error = $state<string | null>(null);
	let hotTrends = $state<HotAllResponse | null>(null);
	let platforms = $state<Platform[]>([]);
	let selectedPlatforms = $state<Set<string>>(new Set());
	let filterMode = $state<'all' | 'selected'>('all');

	async function loadData() {
		loading = true;
		error = null;
		try {
			const [trendsData, platformsData] = await Promise.all([
				apiClient.getAllHotTrends(),
				apiClient.getHotPlatforms()
			]);
			hotTrends = trendsData;
			platforms = platformsData;
		} catch (e) {
			error = e instanceof Error ? e.message : '加载数据失败';
		} finally {
			loading = false;
		}
	}

	function togglePlatform(id: string) {
		const newSet = new Set(selectedPlatforms);
		if (newSet.has(id)) {
			newSet.delete(id);
		} else {
			newSet.add(id);
		}
		selectedPlatforms = newSet;
		filterMode = newSet.size > 0 ? 'selected' : 'all';
	}

	function selectAll() {
		selectedPlatforms = new Set();
		filterMode = 'all';
	}

	function handleSearch(e: CustomEvent<string>) {
		const query = e.detail;
		goto(`/search?q=${encodeURIComponent(query)}`);
	}

	let filteredTrends = $derived(() => {
		if (!hotTrends) return [];
		if (filterMode === 'all' || selectedPlatforms.size === 0) {
			return hotTrends.results;
		}
		return hotTrends.results.filter((t) => selectedPlatforms.has(t.platform_id));
	});

	onMount(() => {
		loadData();
	});
</script>

<svelte:head>
	<title>热榜 - SeeSea</title>
</svelte:head>

<div class="hot-page page-enter">
	<header class="page-header">
		<h1 class="page-title">
			<span class="title-icon">🔥</span>
			实时热榜
		</h1>
		<p class="page-description">聚合各大平台热门内容，一览天下热点</p>
	</header>

	<!-- 平台筛选 -->
	{#if platforms.length > 0}
		<section class="filter-section">
			<div class="filter-header">
				<span class="filter-label">平台筛选：</span>
				<button
					class="filter-btn"
					class:active={filterMode === 'all'}
					onclick={selectAll}
				>
					全部
				</button>
			</div>
			<div class="platform-tags">
				{#each platforms as platform}
					<button
						class="platform-tag"
						class:selected={selectedPlatforms.has(platform.id)}
						onclick={() => togglePlatform(platform.id)}
					>
						{platform.name}
					</button>
				{/each}
			</div>
		</section>
	{/if}

	<!-- 热榜内容 -->
	<section class="content-section">
		{#if loading}
			<Loading text="加载热榜中..." size="lg" />
		{:else if error}
			<ErrorMessage message={error} retry={loadData} />
		{:else if filteredTrends().length > 0}
			<div class="stats-bar">
				<span class="stat">
					共 <strong>{filteredTrends().length}</strong> 个平台
				</span>
				{#if hotTrends}
					<span class="stat">
						成功率 <strong>{Math.round((hotTrends.success_count / (hotTrends.success_count + hotTrends.failed_count)) * 100)}%</strong>
					</span>
				{/if}
			</div>
			<div class="hot-grid">
				{#each filteredTrends() as trend}
					<HotTrendCard {trend} maxItems={15} on:search={handleSearch} />
				{/each}
			</div>
		{:else}
			<EmptyState
				icon="hot"
				title="暂无热榜数据"
				description="稍后再试或检查网络连接"
			/>
		{/if}
	</section>
</div>

<style>
	.hot-page {
		padding: 2rem 0 4rem;
	}

	.page-header {
		text-align: center;
		margin-bottom: 2rem;
	}

	.page-title {
		display: flex;
		align-items: center;
		justify-content: center;
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

	.filter-section {
		background: white;
		border-radius: 12px;
		padding: 1rem 1.25rem;
		margin-bottom: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.filter-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.75rem;
	}

	.filter-label {
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}

	.filter-btn {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		font-weight: 500;
		color: #6b7280;
		background: #f3f4f6;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.filter-btn:hover {
		background: #e5e7eb;
	}

	.filter-btn.active {
		background: #3b82f6;
		color: white;
	}

	.platform-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.platform-tag {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		color: #6b7280;
		background: transparent;
		border: 1px solid #e5e7eb;
		border-radius: 9999px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.platform-tag:hover {
		border-color: #3b82f6;
		color: #3b82f6;
	}

	.platform-tag.selected {
		background: #eff6ff;
		border-color: #3b82f6;
		color: #3b82f6;
	}

	.stats-bar {
		display: flex;
		align-items: center;
		gap: 1.5rem;
		margin-bottom: 1.5rem;
		font-size: 0.875rem;
		color: #6b7280;
	}

	.stat strong {
		color: #1f2937;
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

	/* 移动端适配 */
	@media (max-width: 640px) {
		.hot-page {
			padding: 1rem 0 3rem;
		}

		.page-title {
			font-size: 1.5rem;
		}

		.filter-section {
			padding: 0.75rem 1rem;
		}
	}
</style>
