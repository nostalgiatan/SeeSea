# 教程：第一次搜索

这篇教程带你从零构建 seesea，跑通一次搜索，再读一个页面。做完你会看到结果长什么样，也会知道每个分数来自哪里。需要 20 分钟左右，机器要能上网，并装有 Rust 工具链。

## 1. 构建

在仓库根目录执行：

```bash
cd seesea-core
cargo build --release -p seesea-cli
```

构建结束后，可执行文件在 `target/release/seesea`。后面的命令都假设你在 `seesea-core` 目录里，用 `./target/release/seesea` 运行它。

## 2. 只看摘要的搜索

先用最快的模式搜一次：

```bash
./target/release/seesea search -m fast -n 5 "rust async runtime"
```

屏幕上会出现一个标题行，写着查询词、结果数和耗时，下面是 5 个编号的结果。每个结果有标题、网址，以及一行形如 `score 0.62 · found by duckduckgo#1` 的说明：分数，和哪个来源把它排在第几位。

`fast` 模式只合并各个来源返回的摘要，不下载任何页面，所以很快。

## 3. 读取正文的搜索

去掉 `-m fast`，使用默认的 `standard` 模式：

```bash
./target/release/seesea search -n 5 "rust async runtime"
```

这次 seesea 会下载排名靠前的页面，抽取正文，再按正文重新排序。每个结果下面多了几段文字，这是页面里最能回答查询的章节。耗时会比上一步长，通常要十几秒，取决于最慢的那个页面。

## 4. 看分数的构成

加上 `--json`，同一次搜索会输出 JSON。把它存到文件里：

```bash
./target/release/seesea search --json -n 5 "rust async runtime" > result.json
```

打开 `result.json`，找到 `hits` 数组的第一项。它的 `score_parts` 对象列出了这个分数的各个组成部分：`fusion`、`snippet_overlap`、`title`、`freshness` 属于第一层（只看摘要），`relevance`、`title_support`、`body_quality` 属于第二层（看过正文）。`found_by` 记录哪些来源返回了这个网址。

文件末尾的 `stages` 数组给出各阶段的耗时，`degraded` 数组说明这次搜索里有哪些环节没有正常工作，比如某个来源被拦截。

## 5. 读取一个页面

挑 `result.json` 里任意一个网址，让 seesea 只读这一页，并按问题挑出相关章节：

```bash
./target/release/seesea extract https://tokio.rs/tokio/tutorial/select -q "cancellation" --max-chars 1500
```

输出是这个页面里和 `cancellation` 相关的章节，已经转成 Markdown，导航栏和页脚都被去掉了。

## 6. 检查来源

最后看一眼有哪些来源在工作：

```bash
./target/release/seesea sources --check
```

每个来源会收到一个测试问题，命令列出它是否返回结果、用了多久，失败的会写出原因。验证码拦截在这里显示为 `Blocked`。

## 下一步

- 想加入自己的 SearXNG 或 Brave：[配置搜索来源](../how-to/configure-sources.md)
- 想用本地模型提高排序质量：[准备本地模型](../how-to/set-up-models.md)
- 想知道一次搜索内部发生了什么：[搜索管线如何工作](../explanation/pipeline.md)
