<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.ico';
	import { Navbar, Footer } from '$lib/components';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" type="image/x-icon" href={favicon} />
	<title>SeeSea - 智能搜索引擎</title>
	<meta name="description" content="SeeSea 是一个强大的元搜索引擎，聚合多个搜索引擎结果，提供更全面的搜索体验。" />
</svelte:head>

<div class="app">
	<Navbar />
	<main class="main-content">
		{@render children()}
	</main>
	<Footer />
</div>

<style>
	.app {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		overflow-x: hidden;
	}

	.main-content {
		flex: 1;
		width: 100%;
		max-width: 1280px;
		margin: 0 auto;
		padding: 0 1.5rem;
		overflow-x: hidden;
	}

	@media (max-width: 640px) {
		.main-content {
			padding: 0 1rem;
		}
	}

	@media (max-width: 380px) {
		.main-content {
			padding: 0 0.75rem;
		}
	}
</style>

