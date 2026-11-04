//! Feeds to start from, English and Chinese. They are added on probation: a dead address is
//! switched off after a few failed polls without anyone having to notice.

pub const SEEDS: &[&str] = &[
    // English
    "https://feeds.bbci.co.uk/news/rss.xml",
    "https://feeds.bbci.co.uk/news/technology/rss.xml",
    "https://rss.nytimes.com/services/xml/rss/nyt/HomePage.xml",
    "https://www.theguardian.com/world/rss",
    "https://news.ycombinator.com/rss",
    "https://www.theverge.com/rss/index.xml",
    "https://feeds.arstechnica.com/arstechnica/index",
    "https://techcrunch.com/feed/",
    "https://blog.rust-lang.org/feed.xml",
    "https://www.reddit.com/r/programming/.rss",
    // 中文
    "https://www.chinanews.com.cn/rss/scroll-news.xml",
    "https://sspai.com/feed",
    "https://www.solidot.org/index.rss",
    "https://www.ruanyifeng.com/blog/atom.xml",
    "https://coolshell.cn/feed",
    "https://www.infoq.cn/feed",
    "https://www.ithome.com/rss/",
    "https://rsshub.app/zaobao/znews/china",
];
