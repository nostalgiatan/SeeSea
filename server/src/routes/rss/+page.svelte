<!-- RSS 页面 -->
<script lang="ts">
	import { Loading, ErrorMessage, EmptyState } from '$lib/components';
	import { apiClient, type RssFeedResponse, type RssFeedItem } from '$lib/api';
	import { onMount } from 'svelte';

	let loading = $state(false);
	let error = $state<string | null>(null);
	let feedUrl = $state('');
	let feedResponse = $state<RssFeedResponse | null>(null);
	let templates = $state<string[]>([]);
	let recentFeeds = $state<string[]>([]);

	async function loadTemplates() {
		try {
			templates = await apiClient.getRssTemplates();
		} catch (e) {
			console.error('加载模板失败:', e);
		}
	}

	async function fetchFeed() {
		if (!feedUrl.trim()) return;

		loading = true;
		error = null;

		try {
			feedResponse = await apiClient.fetchRssFeed(feedUrl, { max_items: 50 });
			// 保存到最近使用
			if (!recentFeeds.includes(feedUrl)) {
				recentFeeds = [feedUrl, ...recentFeeds.slice(0, 4)];
				localStorage.setItem('recentFeeds', JSON.stringify(recentFeeds));
			}
		} catch (e) {
			error = e instanceof Error ? e.message : '获取 RSS 内容失败';
			feedResponse = null;
		} finally {
			loading = false;
		}
	}

	function handleSubmit(e: Event) {
		e.preventDefault();
		fetchFeed();
	}

	function selectRecentFeed(url: string) {
		feedUrl = url;
		fetchFeed();
	}

	async function applyTemplate(templateName: string) {
		loading = true;
		error = null;
		
		try {
			const response = await apiClient.addRssTemplate(templateName);
			if (response.success) {
				// 模板应用成功，清空当前数据等待用户输入新的 URL
				feedResponse = null;
				error = null;
			} else {
				error = response.message || '应用模板失败';
			}
		} catch (e) {
			error = e instanceof Error ? e.message : '应用模板失败';
		} finally {
			loading = false;
		}
	}

	function formatDate(dateStr?: string): string {
		if (!dateStr) return '';
		try {
			return new Date(dateStr).toLocaleDateString('zh-CN', {
				year: 'numeric',
				month: 'short',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return dateStr;
		}
	}

	onMount(() => {
		loadTemplates();
		// 加载最近使用的 feeds
		const saved = localStorage.getItem('recentFeeds');
		if (saved) {
			try {
				recentFeeds = JSON.parse(saved);
			} catch {
				recentFeeds = [];
			}
		}
	});
</script>

<svelte:head>
	<title>RSS 订阅 - SeeSea</title>
</svelte:head>

<div class="rss-page page-enter">
	<header class="page-header">
		<h1 class="page-title">
			<span class="title-icon">📡</span>
			RSS 订阅
		</h1>
		<p class="page-description">输入 RSS 链接，获取最新内容</p>
	</header>

	<!-- 输入区域 -->
	<section class="input-section">
		<form onsubmit={handleSubmit} class="feed-form">
			<div class="input-wrapper">
				<input
					type="url"
					bind:value={feedUrl}
					placeholder="输入 RSS Feed URL..."
					class="feed-input"
					disabled={loading}
				/>
				<button type="submit" class="fetch-btn" disabled={loading || !feedUrl.trim()}>
					{#if loading}
						获取中...
					{:else}
						获取内容
					{/if}
				</button>
			</div>
		</form>

		<!-- 最近使用 -->
		{#if recentFeeds.length > 0}
			<div class="recent-feeds">
				<span class="recent-label">最近使用：</span>
				<div class="recent-list">
					{#each recentFeeds as url}
						<button class="recent-item" onclick={() => selectRecentFeed(url)}>
							{new URL(url).hostname}
						</button>
					{/each}
				</div>
			</div>
		{/if}

		<!-- 模板推荐 -->
		{#if templates.length > 0}
			<div class="templates">
				<span class="template-label">推荐模板：</span>
				<div class="template-list">
					{#each templates as template}
						<button 
							class="template-tag" 
							onclick={() => applyTemplate(template)}
							disabled={loading}
						>
							{template}
						</button>
					{/each}
				</div>
			</div>
		{/if}
	</section>

	<!-- 内容区域 -->
	<section class="content-section">
		{#if loading}
			<Loading text="获取 RSS 内容中..." size="lg" />
		{:else if error}
			<ErrorMessage message={error} retry={fetchFeed} />
		{:else if feedResponse}
			<div class="feed-content">
				<!-- Feed 信息 -->
				<div class="feed-meta">
					{#if feedResponse.meta.title}
						<h2 class="feed-title">{feedResponse.meta.title}</h2>
					{/if}
					{#if feedResponse.meta.description}
						<p class="feed-description">{feedResponse.meta.description}</p>
					{/if}
					<span class="item-count">{feedResponse.items.length} 条内容</span>
				</div>

				<!-- 内容列表 -->
				<div class="feed-items">
					{#each feedResponse.items as item, index}
						<article class="feed-item" style="animation-delay: {index * 0.03}s">
							<h3 class="item-title">
								<a href={item.link} target="_blank" rel="noopener noreferrer">
									{item.title}
								</a>
							</h3>
							{#if item.description}
								<p class="item-description">{@html item.description}</p>
							{/if}
							<div class="item-meta">
								{#if item.author}
									<span class="author">{item.author}</span>
								{/if}
								{#if item.published}
									<time class="published">{formatDate(item.published)}</time>
								{/if}
								{#if item.categories.length > 0}
									<div class="categories">
										{#each item.categories.slice(0, 3) as category}
											<span class="category">{category}</span>
										{/each}
									</div>
								{/if}
							</div>
						</article>
					{/each}
				</div>
			</div>
		{:else}
			<EmptyState
				icon="rss"
				title="输入 RSS 链接"
				description="在上方输入框中粘贴 RSS Feed 的 URL"
			/>
		{/if}
	</section>
</div>

<style>
	.rss-page {
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

	.input-section {
		max-width: 680px;
		margin: 0 auto 2rem;
	}

	.feed-form {
		margin-bottom: 1rem;
	}

	.input-wrapper {
		display: flex;
		gap: 0.5rem;
		background: white;
		border: 2px solid #e5e7eb;
		border-radius: 12px;
		padding: 0.5rem;
		transition: all 0.3s;
	}

	.input-wrapper:focus-within {
		border-color: #3b82f6;
		box-shadow: 0 0 0 4px rgba(59, 130, 246, 0.1);
	}

	.feed-input {
		flex: 1;
		padding: 0.625rem 1rem;
		font-size: 1rem;
		border: none;
		outline: none;
		background: transparent;
	}

	.fetch-btn {
		padding: 0.625rem 1.5rem;
		background: linear-gradient(135deg, #3b82f6, #2563eb);
		color: white;
		border: none;
		border-radius: 8px;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}

	.fetch-btn:hover:not(:disabled) {
		background: linear-gradient(135deg, #2563eb, #1d4ed8);
	}

	.fetch-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.recent-feeds,
	.templates {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
		font-size: 0.875rem;
	}

	.recent-feeds {
		margin-bottom: 0.75rem;
	}

	.recent-label,
	.template-label {
		color: #9ca3af;
	}

	.recent-list,
	.template-list {
		display: flex;
		gap: 0.375rem;
		flex-wrap: wrap;
	}

	.recent-item {
		padding: 0.25rem 0.625rem;
		font-size: 0.8125rem;
		color: #6b7280;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.recent-item:hover {
		border-color: #3b82f6;
		color: #3b82f6;
	}

	.template-tag {
		padding: 0.25rem 0.625rem;
		font-size: 0.8125rem;
		color: #6b7280;
		background: #f3f4f6;
		border: 1px solid #e5e7eb;
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.template-tag:hover:not(:disabled) {
		background: #3b82f6;
		color: white;
		border-color: #3b82f6;
		transform: translateY(-1px);
	}

	.template-tag:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.feed-content {
		background: white;
		border-radius: 16px;
		border: 1px solid #e5e7eb;
		overflow: hidden;
	}

	.feed-meta {
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
		background: #f9fafb;
	}

	.feed-title {
		margin: 0 0 0.5rem;
		font-size: 1.25rem;
		font-weight: 600;
		color: #1f2937;
	}

	.feed-description {
		margin: 0 0 0.75rem;
		font-size: 0.9375rem;
		color: #6b7280;
		line-height: 1.6;
	}

	.item-count {
		font-size: 0.8125rem;
		color: #9ca3af;
	}

	.feed-items {
		padding: 0.5rem 0;
	}

	.feed-item {
		padding: 1.25rem 1.5rem;
		border-bottom: 1px solid #f3f4f6;
		animation: fadeIn 0.3s ease-out both;
	}

	.feed-item:last-child {
		border-bottom: none;
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.item-title {
		margin: 0 0 0.5rem;
		font-size: 1rem;
		font-weight: 600;
		line-height: 1.4;
	}

	.item-title a {
		color: #1e40af;
		text-decoration: none;
	}

	.item-title a:hover {
		color: #3b82f6;
		text-decoration: underline;
	}

	.item-description {
		margin: 0 0 0.75rem;
		font-size: 0.875rem;
		color: #6b7280;
		line-height: 1.6;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.item-meta {
		display: flex;
		align-items: center;
		gap: 1rem;
		font-size: 0.75rem;
		color: #9ca3af;
		flex-wrap: wrap;
	}

	.categories {
		display: flex;
		gap: 0.375rem;
	}

	.category {
		padding: 0.125rem 0.5rem;
		background: #f3f4f6;
		border-radius: 4px;
	}

	/* 移动端适配 */
	@media (max-width: 640px) {
		.rss-page {
			padding: 1rem 0 3rem;
		}

		.page-header {
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

		.input-section {
			margin-bottom: 1.5rem;
		}

		.input-wrapper {
			flex-direction: column;
			padding: 0.375rem;
			border-radius: 10px;
		}

		.feed-input {
			padding: 0.5rem 0.75rem;
			font-size: 0.9375rem;
		}

		.fetch-btn {
			width: 100%;
			padding: 0.75rem 1rem;
		}

		.recent-feeds,
		.templates {
			font-size: 0.8125rem;
			gap: 0.375rem;
		}

		.recent-list,
		.template-list {
			gap: 0.25rem;
		}

		.recent-item,
		.template-tag {
			padding: 0.1875rem 0.5rem;
			font-size: 0.75rem;
		}

		.feed-content {
			border-radius: 12px;
		}

		.feed-meta {
			padding: 1rem;
		}

		.feed-title {
			font-size: 1.0625rem;
		}

		.feed-description {
			font-size: 0.8125rem;
			margin-bottom: 0.5rem;
		}

		.item-count {
			font-size: 0.75rem;
		}

		.feed-item {
			padding: 1rem;
		}

		.item-title {
			font-size: 0.9375rem;
			margin-bottom: 0.375rem;
		}

		.item-description {
			font-size: 0.8125rem;
			margin-bottom: 0.5rem;
			-webkit-line-clamp: 3;
			line-clamp: 3;
		}

		.item-meta {
			gap: 0.5rem;
			font-size: 0.6875rem;
		}

		.categories {
			gap: 0.25rem;
		}

		.category {
			padding: 0.0625rem 0.375rem;
			font-size: 0.625rem;
		}
	}

	/* 超小屏适配 */
	@media (max-width: 380px) {
		.rss-page {
			padding: 0.75rem 0 2.5rem;
		}

		.page-title {
			font-size: 1.25rem;
		}

		.page-description {
			font-size: 0.8125rem;
		}

		.input-wrapper {
			padding: 0.25rem;
		}

		.feed-input {
			padding: 0.4375rem 0.625rem;
			font-size: 0.875rem;
		}

		.fetch-btn {
			padding: 0.625rem 0.75rem;
			font-size: 0.875rem;
		}

		.feed-meta {
			padding: 0.75rem;
		}

		.feed-item {
			padding: 0.75rem;
		}

		.item-title {
			font-size: 0.875rem;
		}

		.item-description {
			font-size: 0.75rem;
		}

		.item-meta {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.25rem;
		}
	}
</style>
