<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import StockSearch from '$lib/components/stock/StockSearch.svelte';
	import StockQuoteCard from '$lib/components/stock/StockQuoteCard.svelte';
	import StockDetailPanel from '$lib/components/stock/StockDetailPanel.svelte';
	import MarketOverview from '$lib/components/stock/MarketOverview.svelte';
	import KLineChart from '$lib/components/stock/KLineChart.svelte';
	import StockRank from '$lib/components/stock/StockRank.svelte';
	import SectorHeatMap from '$lib/components/stock/SectorHeatMap.svelte';
	import type { Stock, StockQuote, MarketStatus, IndexQuote } from '$lib/types/stock';

	// 状态
	let selectedStock: Stock | null = $state(null);
	let selectedQuote: StockQuote | null = $state(null);
	let watchlist: Stock[] = $state([]);
	let marketIndices: IndexQuote[] = $state([]);
	let marketStatus: MarketStatus | null = $state(null);
	let showDetailPanel = $state(false);
	let eventSource: EventSource | null = null;
	let isLoading = $state(false);
	let error: string | null = $state(null);

	// SSE 实时行情连接
	function connectRealtime() {
		if (watchlist.length === 0) return;
		
		const codes = watchlist.map(s => s.code).join(',');
		eventSource = new EventSource(`/api/stock/quote/stream?codes=${codes}&interval=5`);
		
		eventSource.onmessage = (event) => {
			try {
				const quotes = JSON.parse(event.data);
				// 更新自选股行情
				quotes.forEach((quote: StockQuote) => {
					const idx = watchlist.findIndex(s => s.code === quote.code);
					if (idx !== -1) {
						// 触发响应式更新
						watchlist[idx] = { ...watchlist[idx], quote };
					}
					// 更新选中股票行情
					if (selectedStock?.code === quote.code) {
						selectedQuote = quote;
					}
				});
			} catch (e) {
				console.error('Parse quote error:', e);
			}
		};
		
		eventSource.onerror = () => {
			eventSource?.close();
			// 5秒后重连
			setTimeout(connectRealtime, 5000);
		};
	}

	// 加载市场概览
	async function loadMarketOverview() {
		try {
			const [indicesRes, statusRes] = await Promise.all([
				fetch('/api/stock/market/indices'),
				fetch('/api/stock/market/status')
			]);
			
			if (indicesRes.ok) {
				marketIndices = await indicesRes.json();
			}
			if (statusRes.ok) {
				marketStatus = await statusRes.json();
			}
		} catch (e) {
			console.error('Load market overview failed:', e);
		}
	}

	// 选择股票
	async function selectStock(stock: Stock) {
		selectedStock = stock;
		isLoading = true;
		error = null;
		
		try {
			// 获取实时行情
			const quoteRes = await fetch(`/api/stock/quote?codes=${stock.code}`);
			if (quoteRes.ok) {
				const quotes = await quoteRes.json();
				selectedQuote = quotes[0] || null;
			}
			
			showDetailPanel = true;
		} catch (e) {
			error = '获取股票数据失败';
			console.error(e);
		} finally {
			isLoading = false;
		}
	}

	// 添加自选
	function addToWatchlist(stock: Stock) {
		if (!watchlist.find(s => s.code === stock.code)) {
			watchlist = [...watchlist, stock];
			// 重新连接实时行情
			eventSource?.close();
			connectRealtime();
		}
	}

	// 移除自选
	function removeFromWatchlist(code: string) {
		watchlist = watchlist.filter(s => s.code !== code);
		if (watchlist.length > 0) {
			eventSource?.close();
			connectRealtime();
		} else {
			eventSource?.close();
		}
	}

	// 关闭详情面板
	function closeDetailPanel() {
		showDetailPanel = false;
	}

	onMount(() => {
		loadMarketOverview();
		// 定时刷新市场概览
		const interval = setInterval(loadMarketOverview, 30000);
		
		return () => {
			clearInterval(interval);
		};
	});

	onDestroy(() => {
		eventSource?.close();
	});

	// 从排行榜选择股票
	function handleRankSelect(code: string, name: string) {
		selectStock({ code, name });
	}

	// 点击板块
	function handleSectorClick(code: string, name: string) {
		// 可以跳转到板块详情页或展示板块成分股
		console.log('Sector clicked:', code, name);
	}
</script>

<svelte:head>
	<title>股票行情 - SeeSea</title>
	<meta name="description" content="实时股票行情、K线图表、财务分析" />
</svelte:head>

<div class="stock-page">
	<!-- 市场概览 -->
	<MarketOverview {marketIndices} {marketStatus} />

	<!-- 板块热力图 -->
	<div class="heatmap-section">
		<SectorHeatMap onSectorClick={handleSectorClick} />
	</div>

	<div class="stock-content">
		<!-- 左侧：搜索和自选股 -->
		<div class="stock-sidebar">
			<!-- 搜索框 -->
			<StockSearch onSelect={selectStock} onAddWatchlist={addToWatchlist} />

			<!-- 涨跌排行 -->
			<StockRank onSelect={handleRankSelect} />

			<!-- 自选股列表 -->
			<div class="watchlist-section">
				<h3 class="section-title">
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon>
					</svg>
					自选股
					<span class="count">({watchlist.length})</span>
				</h3>
				
				{#if watchlist.length === 0}
					<div class="empty-watchlist">
						<p>暂无自选股</p>
						<p class="hint">搜索股票并点击 ⭐ 添加</p>
					</div>
				{:else}
					<div class="watchlist">
						{#each watchlist as stock (stock.code)}
							<StockQuoteCard 
								{stock}
								quote={stock.quote}
								isSelected={selectedStock?.code === stock.code}
								onClick={() => selectStock(stock)}
								onRemove={() => removeFromWatchlist(stock.code)}
							/>
						{/each}
					</div>
				{/if}
			</div>
		</div>

		<!-- 右侧：详情面板 -->
		<div class="stock-main" class:has-selection={showDetailPanel}>
			{#if showDetailPanel && selectedStock}
				<StockDetailPanel
					stock={selectedStock}
					quote={selectedQuote}
					{isLoading}
					{error}
					onClose={closeDetailPanel}
					onAddWatchlist={() => addToWatchlist(selectedStock!)}
				/>
			{:else}
				<div class="stock-placeholder">
					<div class="placeholder-content">
						<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline>
						</svg>
						<h2>选择一只股票查看详情</h2>
						<p>搜索股票代码或名称，或从自选股列表中选择</p>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>

<style>
	.stock-page {
		padding: 1.5rem 0;
		min-height: calc(100vh - 180px);
	}

	.stock-content {
		display: grid;
		grid-template-columns: 360px 1fr;
		gap: 1.5rem;
		margin-top: 1.5rem;
	}

	.heatmap-section {
		margin-top: 1.5rem;
	}

	.stock-sidebar {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.watchlist-section {
		background: var(--card-bg, #fff);
		border-radius: 12px;
		padding: 1rem;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
	}

	.section-title {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 1rem;
		font-weight: 600;
		margin-bottom: 1rem;
		color: var(--text-primary, #1a1a1a);
	}

	.section-title .count {
		font-weight: 400;
		color: var(--text-secondary, #666);
	}

	.empty-watchlist {
		text-align: center;
		padding: 2rem 1rem;
		color: var(--text-secondary, #666);
	}

	.empty-watchlist p {
		margin: 0.5rem 0;
	}

	.empty-watchlist .hint {
		font-size: 0.875rem;
		opacity: 0.8;
	}

	.watchlist {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: 500px;
		overflow-y: auto;
	}

	.stock-main {
		min-height: 600px;
	}

	.stock-placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		min-height: 500px;
		background: var(--card-bg, #fff);
		border-radius: 12px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
	}

	.placeholder-content {
		text-align: center;
		color: var(--text-secondary, #666);
	}

	.placeholder-content svg {
		opacity: 0.3;
		margin-bottom: 1.5rem;
	}

	.placeholder-content h2 {
		font-size: 1.25rem;
		font-weight: 500;
		margin-bottom: 0.5rem;
		color: var(--text-primary, #1a1a1a);
	}

	.placeholder-content p {
		font-size: 0.875rem;
	}

	/* 响应式布局 */
	@media (max-width: 1024px) {
		.stock-content {
			grid-template-columns: 1fr;
		}

		.stock-sidebar {
			order: 1;
		}

		.stock-main {
			order: 0;
		}

		.stock-main:not(.has-selection) {
			display: none;
		}
	}

	@media (max-width: 640px) {
		.stock-page {
			padding: 1rem 0;
		}

		.stock-content {
			gap: 1rem;
		}

		.watchlist-section {
			padding: 0.75rem;
		}
	}

	/* 暗色主题 */
	:global(.dark) .stock-page {
		--card-bg: #1e1e1e;
		--text-primary: #e0e0e0;
		--text-secondary: #999;
	}
</style>
