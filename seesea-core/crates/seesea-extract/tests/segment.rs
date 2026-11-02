use seesea_extract::{blocks, pagefeat};

const PAGE: &str = r#"<html lang="zh"><head><title>T</title></head><body>
<nav><a href="/">首页</a></nav>
<article>
<h2>小标题</h2>
<p>这是<strong>加粗</strong>的文字，<a href="https://example.com/x">链接</a>在句中，以及 <code>code</code> 片段。</p>
<ul><li>第一项</li><li>第二项</li></ul>
<blockquote><p>引用的话</p></blockquote>
<pre>fn main() {
    println!("hi");
}</pre>
</article>
<script>var x = 1;</script>
</body></html>"#;

fn md_of(needle: &str) -> String {
    blocks::segment(PAGE).into_iter().map(|b| b.md).find(|m| m.contains(needle)).unwrap_or_else(|| panic!("no block containing {needle}"))
}

#[test]
fn inline_markup_stays_in_its_sentence() {
    let m = md_of("加粗");
    assert_eq!(m, "这是**加粗**的文字， [链接](https://example.com/x) 在句中，以及 `code` 片段。".replace("， [", "，[").replace(") 在", ")在"), "got {m}");
}

#[test]
fn block_prefixes() {
    assert_eq!(md_of("小标题"), "## 小标题");
    assert_eq!(md_of("第二项"), "- 第二项");
    assert_eq!(md_of("引用的话"), "> 引用的话");
    assert!(md_of("println").starts_with("```\n") && md_of("println").contains("    println!"));
}

#[test]
fn scripts_are_not_blocks() {
    assert!(blocks::segment(PAGE).iter().all(|b| !b.text.contains("var x")));
}

#[test]
fn page_feature_width_matches_constant() {
    let b = blocks::segment(PAGE);
    assert_eq!(pagefeat::page_features("https://example.com/a/b", PAGE, &b).len(), pagefeat::N_PAGE_FEATURES);
}

#[test]
fn feature_width_matches_names() {
    let b = blocks::segment(PAGE);
    assert!(b.iter().all(|x| x.feats.len() == blocks::feature_names().len()));
}

#[test]
fn bundled_model_extracts_the_article_not_the_chrome() {
    let body = "这是一段足够长的正文内容，用来测试内置模型能否把文章主体从导航和页脚里分出来。".repeat(6);
    let html = format!(r#"<html lang="zh"><head><title>测试文章</title><meta name="description" content="摘要"></head><body>
<nav><ul><li><a href="/a">首页</a></li><li><a href="/b">关于</a></li><li><a href="/c">联系</a></li></ul></nav>
<main><article><h1>测试文章</h1><p>{body}</p><p>{body}</p></article></main>
<footer><a href="/p">隐私政策</a> <a href="/t">服务条款</a> 版权所有</footer></body></html>"#);
    let r = seesea_extract::Extractor::new().extract(&html, "https://example.com/post/1", &Default::default());
    assert!(r.core.contains("足够长的正文内容"), "core: {}", r.core);
    assert!(!r.core.contains("隐私政策"), "core: {}", r.core);
    assert_eq!(r.meta.title.as_deref(), Some("测试文章"));
    assert_eq!(r.meta.language.as_deref(), Some("zh"));
    assert!(!r.needs_render && r.confidence > 0.5);
}

#[test]
fn near_empty_js_shell_asks_for_rendering() {
    let html = r#"<html><head><title>App</title></head><body><div id="root"></div><script src="/app.js"></script></body></html>"#;
    let r = seesea_extract::Extractor::new().extract(html, "https://example.com/", &Default::default());
    assert!(r.needs_render);
}
