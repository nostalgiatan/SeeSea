#!/usr/bin/env python3
"""
Analyze current Google HTML structure to understand parsing requirements
"""

import requests
import re
import html.parser

def analyze_google_html():
    print("=== Analyzing Current Google HTML Structure ===")

    # Try a very simple request first
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
        'Accept-Language': 'en-US,en;q=0.5',
        'Accept-Encoding': 'gzip, deflate',
        'DNT': '1',
        'Connection': 'keep-alive',
        'Upgrade-Insecure-Requests': '1'
    }

    cookies = {
        'CONSENT': 'YES+cb.20210301-17-p0.en-GB+FX+700'
    }

    # Test simple search without advanced parameters
    url = "https://www.google.com/search?q=python+programming&num=10"

    print(f"Testing URL: {url}")

    try:
        response = requests.get(url, headers=headers, cookies=cookies, timeout=10)
        print(f"Status: {response.status_code}")
        print(f"Final URL: {response.url}")
        print(f"Content length: {len(response.text)}")

        if "sorry.google.com" in response.url or response.status_code != 200:
            print("❌ CAPTCHA or error detected")
            return None

        # Use regex patterns to analyze HTML
        html_content = response.text

        # Look for various result container patterns using regex
        patterns_to_test = [
            # Modern Google patterns
            {'name': 'jscontroller SC7lYd', 'pattern': r'<div[^>]*jscontroller="SC7lYd"[^>]*>'},
            {'name': 'data-hveid', 'pattern': r'<div[^>]*data-hveid=[^>]*>'},
            {'name': 'class with g', 'pattern': r'<div[^>]*class="[^"]*g[^"]*"[^>]*>'},
            {'name': 'class Gx5Zad', 'pattern': r'<div[^>]*class="[^"]*Gx5Zad[^"]*"[^>]*>'},
            {'name': 'class with MjjYud', 'pattern': r'<div[^>]*class="[^"]*MjjYud[^"]*"[^>]*>'},
            {'name': 'class with yuRUbf', 'pattern': r'<div[^>]*class="[^"]*yuRUbf[^"]*"[^>]*>'},
            # Legacy patterns
            {'name': 'class with rc', 'pattern': r'<div[^>]*class="[^"]*rc[^"]*"[^>]*>'},
            {'name': 'class with srg', 'pattern': r'<div[^>]*class="[^"]*srg[^"]*"[^>]*>'},
        ]

        print("\n=== Result Container Analysis ===")
        found_containers = []

        for pattern in patterns_to_test:
            matches = re.findall(pattern['pattern'], html_content, re.IGNORECASE)
            if matches:
                print(f"✅ Found {len(matches)} elements with pattern: {pattern['name']}")
                found_containers.append(pattern)
                print(f"   Sample: {matches[0][:100]}...")
            else:
                print(f"❌ No elements found for pattern: {pattern['name']}")

        # Look for title patterns
        print("\n=== Title Analysis ===")
        title_patterns = [
            r'<h3[^>]*>(.*?)</h3>',
            r'<a[^>]*><h3[^>]*>(.*?)</h3></a>',
        ]

        for pattern in title_patterns:
            matches = re.findall(pattern, html_content, re.IGNORECASE | re.DOTALL)
            if matches:
                print(f"✅ Found {len(matches)} titles")
                # Clean up the first match
                clean_title = re.sub(r'<[^>]+>', '', matches[0]).strip()
                if clean_title and len(clean_title) > 3:
                    print(f"   Sample title: {clean_title[:50]}...")
                    break

        # Look for URL patterns
        print("\n=== URL Analysis ===")
        url_patterns = [
            r'<a[^>]*href="(/url[^"]*|http[^"]*)"[^>]*><h3',
            r'href="(https?://[^"]*)"',
        ]

        for pattern in url_patterns:
            matches = re.findall(pattern, html_content, re.IGNORECASE)
            if matches:
                print(f"✅ Found {len(matches)} URLs")
                filtered_urls = [url for url in matches if not url.startswith('/url?q=') or 'google.com' not in url]
                if filtered_urls:
                    print(f"   Sample URL: {filtered_urls[0][:50]}...")
                    break

        # Look for snippet patterns
        print("\n=== Snippet Analysis ===")
        snippet_patterns = [
            {'name': 'data-sncf', 'pattern': r'<div[^>]*data-sncf="1"[^>]*>(.*?)</div>'},
            {'name': 'class VwiC3b', 'pattern': r'<div[^>]*class="[^"]*VwiC3b[^"]*"[^>]*>(.*?)</div>'},
            {'name': 'class aCOpRe', 'pattern': r'<span[^>]*class="[^"]*aCOpRe[^"]*"[^>]*>(.*?)</span>'},
            {'name': 'class s', 'pattern': r'<div[^>]*class="[^"]*s[^"]*"[^>]*>(.*?)</div>'},
        ]

        for pattern in snippet_patterns:
            matches = re.findall(pattern['pattern'], html_content, re.IGNORECASE | re.DOTALL)
            if matches:
                print(f"✅ Found {len(matches)} snippets with pattern: {pattern['name']}")
                # Clean up the first match
                clean_snippet = re.sub(r'<[^>]+>', '', matches[0]).strip()
                if clean_snippet and len(clean_snippet) > 20:
                    print(f"   Sample snippet: {clean_snippet[:100]}...")
                    break

        # Save HTML for further analysis
        with open('./google_response.html', 'w', encoding='utf-8') as f:
            f.write(response.text)
        print(f"\n📄 Full HTML saved to ./google_response.html")

        return {
            'found_containers': found_containers,
            'total_content_length': len(response.text),
            'status_code': response.status_code,
            'final_url': response.url
        }

    except Exception as e:
        print(f"❌ Error: {e}")
        return None

if __name__ == "__main__":
    result = analyze_google_html()

    if result:
        print("\n=== Summary ===")
        print(f"Status Code: {result['status_code']}")
        print(f"Final URL: {result['final_url']}")
        print(f"Content Length: {result['total_content_length']}")
        print(f"Working Patterns: {len(result['found_containers'])}")

        if result['found_containers']:
            print("\n✅ Successfully identified Google result patterns!")
            print("Update your Rust selectors to use the working patterns found above.")
        else:
            print("\n❌ No working patterns found - Google may be blocking requests")