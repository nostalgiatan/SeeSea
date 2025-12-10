import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		// 使用静态适配器构建纯静态网站，使用 SPA 模式
		adapter: adapter({
			// 输出目录
			out: 'build',
			// SPA 模式：未找到的路由返回 index.html
			fallback: 'index.html',
			// 启用压缩
			precompress: true
		})
		// 不设置 base 路径，使用根路径
	}
};

export default config;
