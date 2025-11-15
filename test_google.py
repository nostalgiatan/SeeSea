#!/usr/bin/env python3
"""
Test script to analyze Google search behavior and compare with SearXNG implementation
"""

import requests
import re
import sys
from urllib.parse import urlencode

# Test basic Google search like SearXNG does
def test_google_search():
    print("=== Testing Google Search ===")

    # Simulate SearXNG's Google request
    headers = {
        'Accept': '*/*',
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
    }

    cookies = {
        'CONSENT': 'YES+'
    }

    # Test query
    query = "python programming"

    # Build URL like SearXNG does
    params = {
        'q': query,
        'hl': 'en-US',
        'lr': 'lang_en',
        'cr': 'countryUS',
        'ie': 'utf8',
        'oe': 'utf8',
        'filter': '0',
        'start': 0,
        'asearch': 'arc',
        'async': 'arc_id:srp_test12345_100,use_ac:true,_fmt:prog'
    }

    url = f"https://www.google.com/search?{urlencode(params)}"

    print(f"Request URL: {url}")
    print(f"Headers: {headers}")
    print(f"Cookies: {cookies}")

    try:
        response = requests.get(url, headers=headers, cookies=cookies, timeout=10)
        print(f"Status Code: {response.status_code}")
        print(f"Final URL: {response.url}")
        print(f"Content Length: {len(response.text)}")

        # Check for CAPTCHA
        if "sorry.google.com" in response.url or "/sorry" in response.url:
            print("❌ DETECTED GOOGLE CAPTCHA")
            return False

        # Check for results
        if "did not match any documents" in response.text:
            print("❌ NO RESULTS FOUND")
            return False

        # Look for result containers
        result_patterns = [
            r'<div[^>]*class="[^"]*g[^"]*"[^>]*>',
            r'<div[^>]*data-hveid=',
            r'<div[^>]*class="[^"]*Gx5Zad[^"]*"[^>]*>',
            r'<div[^>]*jscontroller="SC7lYd"[^>]*>'
        ]

        found_results = 0
        for pattern in result_patterns:
            matches = re.findall(pattern, response.text)
            if matches:
                print(f"✅ Found {len(matches)} results with pattern: {pattern}")
                found_results += len(matches)

                # Extract first result details
                if len(matches) > 0:
                    print("\n--- First Result Analysis ---")

                    # Extract title
                    title_match = re.search(r'<h3[^>]*>(.*?)</h3>', response.text, re.DOTALL)
                    if title_match:
                        title = re.sub(r'<[^>]+>', '', title_match.group(1)).strip()
                        print(f"Title: {title}")

                    # Extract URL
                    url_match = re.search(r'<a[^>]*href="([^"]+)"[^>]*><h3', response.text)
                    if url_match:
                        url = url_match.group(1)
                        print(f"URL: {url}")

                    # Extract snippet
                    snippet_patterns = [
                        r'<div[^>]*data-sncf="1"[^>]*>(.*?)</div>',
                        r'<div[^>]*class="[^"]*VwiC3b[^"]*"[^>]*>(.*?)</div>',
                        r'<span[^>]*class="[^"]*aCOpRe[^"]*"[^>]*>(.*?)</span>'
                    ]

                    for snippet_pattern in snippet_patterns:
                        snippet_match = re.search(snippet_pattern, response.text, re.DOTALL)
                        if snippet_match:
                            snippet = re.sub(r'<[^>]+>', '', snippet_match.group(1)).strip()
                            if len(snippet) > 20:  # Only show meaningful snippets
                                print(f"Snippet: {snippet[:200]}...")
                                break

        if found_results == 0:
            print("❌ NO RESULT CONTAINERS FOUND")
            print("\n--- HTML Preview ---")
            print(response.text[:1000])
            return False
        else:
            print(f"✅ FOUND {found_results} POTENTIAL RESULTS")
            return True

    except Exception as e:
        print(f"❌ ERROR: {e}")
        return False

def test_simple_google_search():
    print("\n=== Testing Simple Google Search ===")

    try:
        # Very simple request
        url = "https://www.google.com/search?q=python+programming"
        headers = {
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
        }

        response = requests.get(url, headers=headers, timeout=10)
        print(f"Status Code: {response.status_code}")
        print(f"Content Length: {len(response.text)}")

        if "sorry.google.com" in response.url:
            print("❌ CAPTCHA detected even with simple request")
            return False

        # Check for basic result indicators
        if 'class="g"' in response.text or 'data-hveid=' in response.text:
            print("✅ Found basic result containers")
            return True
        else:
            print("❌ No basic result containers found")
            return False

    except Exception as e:
        print(f"❌ ERROR: {e}")
        return False

if __name__ == "__main__":
    print("Google Search Analysis")
    print("=" * 50)

    # Test simple search first
    simple_works = test_simple_google_search()

    # Test SearXNG-style search
    searxng_works = test_google_search()

    print("\n" + "=" * 50)
    print("SUMMARY:")
    print(f"Simple Google Search: {'✅ WORKS' if simple_works else '❌ FAILED'}")
    print(f"SearXNG-style Search: {'✅ WORKS' if searxng_works else '❌ FAILED'}")

    if not simple_works and not searxng_works:
        print("\n🔍 Both methods failed - likely blocked by Google")
        print("💡 Consider:")
        print("   - Using different IP addresses")
        print("   - Rotating user agents")
        print("   - Adding delays between requests")
        print("   - Using proxy services")
    elif simple_works and not searxng_works:
        print("\n🔍 Simple search works but SearXNG-style doesn't")
        print("💡 The issue is likely in the advanced parameters or parsing")
    elif not simple_works and searxng_works:
        print("\n🔍 SearXNG-style works but simple doesn't")
        print("💡 Unusual - might be rate limiting")
    else:
        print("\n🔍 Both methods work - issue might be in Rust implementation details")