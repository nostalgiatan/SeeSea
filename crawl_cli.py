#!/usr/bin/env python3
"""
Command-line interface for WebCrawler

This script provides a command-line interface to the WebCrawler module,
allowing users to crawl web pages and save the rendered HTML to files.

Usage:
    python crawl_cli.py <url> [options]
    python crawl_cli.py https://example.com --output example.html --wait-until domcontentloaded

Options:
    -h, --help              Show this help message and exit
    -o, --output FILE       Output file name (default: output.html)
    -b, --browser BROWSER   Browser type (chromium, firefox, webkit; default: chromium)
    -H, --headless          Run browser in headless mode (default: True)
    -w, --wait-until WAIT   Wait condition (domcontentloaded, load, networkidle; default: domcontentloaded)
    -a, --additional-wait MS Additional wait time in milliseconds after page load
    -t, --timeout MS        Timeout in milliseconds (default: 30000)
    -u, --user-agent UA     Custom user agent string
"""

import argparse
import asyncio
import sys
from seesea.Pro import WebCrawler


def parse_args():
    """
    Parse command line arguments
    """
    parser = argparse.ArgumentParser(
        description="Command-line interface for WebCrawler",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )

    # Required positional argument
    parser.add_argument("url", help="URL to crawl")

    # Optional arguments
    parser.add_argument(
        "-o", "--output", default="output.html", help="Output file name (default: output.html)"
    )

    parser.add_argument(
        "-b",
        "--browser",
        choices=["chromium", "firefox", "webkit"],
        default="chromium",
        help="Browser type (chromium, firefox, webkit; default: chromium)",
    )

    parser.add_argument(
        "-H",
        "--headless",
        action="store_true",
        default=True,
        help="Run browser in headless mode (default: True)",
    )

    parser.add_argument(
        "--no-headless",
        action="store_false",
        dest="headless",
        help="Run browser in non-headless mode",
    )

    parser.add_argument(
        "-w",
        "--wait-until",
        choices=["domcontentloaded", "load", "networkidle"],
        default="domcontentloaded",
        help="Wait condition (domcontentloaded, load, networkidle; default: domcontentloaded)",
    )

    parser.add_argument(
        "-a",
        "--additional-wait",
        type=int,
        default=0,
        help="Additional wait time in milliseconds after page load",
    )

    parser.add_argument(
        "-t", "--timeout", type=int, default=30000, help="Timeout in milliseconds (default: 30000)"
    )

    parser.add_argument("-u", "--user-agent", default=None, help="Custom user agent string")

    return parser.parse_args()


async def crawl_url(args):
    """
    Crawl the specified URL and save the result to a file
    """
    print(f"🚀 Crawling: {args.url}")
    print("🔧 Configuration:")
    print(f"   Browser: {args.browser}")
    print(f"   Headless: {args.headless}")
    print(f"   Wait Until: {args.wait_until}")
    print(f"   Additional Wait: {args.additional_wait}ms")
    print(f"   Timeout: {args.timeout}ms")
    print(f"   Output: {args.output}")

    if args.user_agent:
        print(f"   User Agent: {args.user_agent[:50]}...")

    print("\n⏳ Starting crawl...")

    try:
        # Create crawler instance
        crawler = WebCrawler(
            headless=args.headless,
            browser_type=args.browser,
            timeout=args.timeout,
            user_agent=args.user_agent,
        )

        # Crawl the URL
        html = await crawler.crawl(
            args.url, wait_until=args.wait_until, additional_wait=args.additional_wait
        )

        # Close the crawler
        await crawler.close()

        # Save the HTML to file
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(html)

        print("✅ Crawl completed successfully!")
        print(f"📁 HTML saved to: {args.output}")
        print(f"📊 HTML length: {len(html):,} characters")

        return 0

    except Exception as e:
        print(f"❌ Crawl failed: {e}")
        import traceback

        traceback.print_exc()
        return 1


def main():
    """
    Main function
    """
    # Parse arguments
    args = parse_args()

    # Run the crawl
    return asyncio.run(crawl_url(args))


if __name__ == "__main__":
    sys.exit(main())
