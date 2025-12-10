<!-- 搜索页面 -->
<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { SearchBox, SearchResultCard, Loading, ErrorMessage, EmptyState } from '$lib/components';
	import { apiClient, type SearchResponse } from '$lib/api';
	import { onMount } from 'svelte';

	let searchQuery = $state('');
	let loading = $state(false);
	let error = $state<string | null>(null);
	let searchResponse = $state<SearchResponse | null>(null);
	let useProSearch = $state(false);

	// 从 URL 获取查询参数
	let urlQuery = $derived($page.url.searchParams.get('q') || '');

	async function performSearch(query: string) {
		if (!query.trim()) return;

		loading = true;
		error = null;

		try {
			if (useProSearch) {
				searchResponse = await apiClient.proSearch(query, { page_size: 30 });
			} else {
				searchResponse = await apiClient.search(query, { page_size: 30 });
			}
		} catch (e) {
			error = e instanceof Error ? e.message : '搜索失败';
			searchResponse = null;
		} finally {
			loading = false;
		}
	}

	function handleSearch(e: CustomEvent<string>) {
		const query = e.detail;
		searchQuery = query;
		goto(`/search?q=${encodeURIComponent(query)}`, { replaceState: true });
		performSearch(query);
	}

	function handleClear() {
		searchQuery = '';
		searchResponse = null;
		goto('/search', { replaceState: true });
	}

	function toggleProSearch() {
		useProSearch = !useProSearch;
		if (searchQuery) {
			performSearch(searchQuery);
		}
	}

	// 监听 URL 变化
	$effect(() => {
		if (urlQuery && urlQuery !== searchQuery) {
			searchQuery = urlQuery;
			performSearch(urlQuery);
		}
	});

	onMount(() => {
		if (urlQuery) {
			searchQuery = urlQuery;
			performSearch(urlQuery);
		}
	});
</script>

<svelte:head>
	<title>{searchQuery ? `${searchQuery} - SeeSea 搜索` : 'SeeSea 搜索'}</title>
</svelte:head>

<div class="search-page page-enter">
	<!-- 搜索区域 -->
	<section class="search-section">
		<div class="search-header">
			<SearchBox
				bind:value={searchQuery}
				{loading}
				on:search={handleSearch}
				on:clear={handleClear}
			/>
			<div class="search-options">
				<button
					class="option-btn"
					class:active={useProSearch}
					onclick={toggleProSearch}
					title="Pro 搜索使用向量检索进行智能排序"
				>
					<span class="option-icon">🤖</span>
					Pro 搜索
				</button>
			</div>
		</div>

		{#if searchResponse}
			<div class="search-meta">
				<span class="result-count">
					找到约 <strong>{searchResponse.total_count.toLocaleString()}</strong> 条结果
				</span>
				<span class="search-time">
					用时 {searchResponse.query_time_ms} ms
				</span>
				{#if searchResponse.cached}
					<span class="cache-badge">缓存</span>
				{/if}
				<span class="engines-used">
					引擎: {searchResponse.engines_used.join(', ')}
				</span>
			</div>
		{/if}
	</section>

	<!-- 结果区域 -->
	<section class="results-section">
		{#if loading}
			<Loading text="搜索中..." size="lg" />
		{:else if error}
			<ErrorMessage message={error} retry={() => performSearch(searchQuery)} />
		{:else if searchResponse && searchResponse.results.length > 0}
			<div class="results-list">
				{#each searchResponse.results as result, index}
					<SearchResultCard {result} {index} />
				{/each}
			</div>
		{:else if searchQuery && !loading}
			<EmptyState
				icon="search"
				title="没有找到相关结果"
				description="尝试使用不同的关键词，或者检查拼写是否正确"
			/>
		{:else}
			<EmptyState
				icon="search"
				title="开始搜索"
				description="在上方输入框中输入你想搜索的内容"
			/>
		{/if}
	</section>
</div>

<style>
	.search-page {
		padding: 2rem 0 4rem;
	}

	.search-section {
		margin-bottom: 1.5rem;
	}

	.search-header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		align-items: center;
	}

	.search-options {
		display: flex;
		gap: 0.5rem;
	}

	.option-btn {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #6b7280;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 9999px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.option-btn:hover {
		border-color: #3b82f6;
		color: #3b82f6;
	}

	.option-btn.active {
		background: linear-gradient(135deg, #3b82f6, #2563eb);
		color: white;
		border-color: transparent;
	}

	.option-icon {
		font-size: 1rem;
	}

	.search-meta {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		margin-top: 1rem;
		padding: 0.75rem 1rem;
		background: white;
		border-radius: 8px;
		font-size: 0.8125rem;
		color: #6b7280;
		flex-wrap: wrap;
	}

	.result-count strong {
		color: #1f2937;
	}

	.search-time {
		color: #9ca3af;
	}

	.cache-badge {
		padding: 0.125rem 0.5rem;
		background: #dcfce7;
		color: #16a34a;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.engines-used {
		color: #9ca3af;
	}

	.results-section {
		min-height: 400px;
	}

	.results-list {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	/* 移动端适配 */
	@media (max-width: 640px) {
		.search-page {
			padding: 1rem 0 3rem;
		}

		.search-meta {
			gap: 0.5rem;
			font-size: 0.75rem;
		}
	}
</style>
