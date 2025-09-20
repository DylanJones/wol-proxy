#!/usr/bin/env python3
"""
web_search.py — Simple web fetcher for agents

This script provides a minimal, dependable "web search" utility similar in spirit to AI agent tools:
- Fetches a web page by URL
- Optionally extracts the main content (readability-like) and/or returns raw HTML
- Supports returning JSON with normalized fields for other agents to consume
- Optional query string to highlight/restrict content

Usage:
  web_search.py --url https://example.com --format json
  web_search.py --url https://example.com --query "keyword" --format text
  web_search.py --url https://example.com --raw --format text

Exit codes:
  0 success
  2 bad usage
  3 network error
  4 non-200 HTTP status
  5 parse error

Note: This avoids heavy dependencies. It uses requests + BeautifulSoup only.
"""

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from typing import Optional

try:
    import requests
    from bs4 import BeautifulSoup
except Exception as e:
    print("Missing dependencies. Install with: pip install requests beautifulsoup4", file=sys.stderr)
    sys.exit(2)


@dataclass
class Result:
    url: str
    status: int
    title: Optional[str]
    text: Optional[str]
    html: Optional[str]


def extract_main_text(html: str) -> str:
    soup = BeautifulSoup(html, "html.parser")

    # Remove common noise
    for tag in soup(["script", "style", "noscript", "iframe", "svg", "header", "footer", "nav"]):
        tag.decompose()

    # Prefer <main>, then article, then body
    main = soup.find("main") or soup.find("article") or soup.body or soup

    # Join paragraph-like text
    parts = []
    for p in main.find_all(["h1", "h2", "h3", "h4", "h5", "h6", "p", "li"], recursive=True):
        text = p.get_text(" ", strip=True)
        if text:
            parts.append(text)

    text = "\n".join(parts)

    # Normalize whitespace
    text = re.sub(r"\s+", " ", text)
    text = re.sub(r"\s*\n\s*", "\n", text)
    return text.strip()


def fetch(url: str, want_raw_html: bool) -> Result:
    try:
        resp = requests.get(url, timeout=20, headers={
            "User-Agent": "wol-proxy-agent-websearch/1.0 (+https://github.com/DylanJones/wol-proxy)",
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        })
    except requests.RequestException as e:
        print(f"Network error: {e}", file=sys.stderr)
        sys.exit(3)

    if resp.status_code != 200:
        print(f"Non-200 status: {resp.status_code}", file=sys.stderr)
        return Result(url=url, status=resp.status_code, title=None, text=None, html=resp.text if want_raw_html else None)

    html = resp.text
    title = None
    try:
        soup = BeautifulSoup(html, "html.parser")
        if soup.title and soup.title.string:
            title = soup.title.string.strip()
    except Exception:
        pass

    text = None
    try:
        text = extract_main_text(html)
    except Exception as e:
        print(f"Parse error: {e}", file=sys.stderr)
        # Continue; leave text None

    return Result(url=url, status=resp.status_code, title=title, text=text, html=html if want_raw_html else None)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Minimal web fetcher for agents")
    parser.add_argument("--url", required=True, help="URL to fetch")
    parser.add_argument("--query", help="Optional query to filter/highlight text output")
    parser.add_argument("--raw", action="store_true", help="Include raw HTML in the result")
    parser.add_argument("--format", choices=["json", "text"], default="json", help="Output format")
    parser.add_argument("--max-chars", type=int, default=20000, help="Trim text/html to at most N characters")

    args = parser.parse_args(argv)

    result = fetch(args.url, want_raw_html=args.raw)

    # Optionally filter by query for text output
    if args.query and result.text:
        pattern = re.compile(re.escape(args.query), re.IGNORECASE)
        # Keep sentences containing the query (simple heuristic)
        sentences = re.split(r"(?<=[.!?])\s+", result.text)
        hits = [s for s in sentences if pattern.search(s)]
        filtered = " ".join(hits)[: args.max_chars]
        if filtered:
            result.text = filtered

    # Truncate
    if result.text and len(result.text) > args.max_chars:
        result.text = result.text[: args.max_chars] + "…"
    if result.html and len(result.html) > args.max_chars:
        result.html = result.html[: args.max_chars] + "…"

    if args.format == "json":
        print(json.dumps(asdict(result), ensure_ascii=False))
    else:
        # text format: show title and a snippet of text
        lines = []
        if result.title:
            lines.append(f"# {result.title}")
        lines.append(f"URL: {result.url} (status {result.status})")
        if result.text:
            lines.append("")
            lines.append(result.text)
        elif result.html:
            lines.append("")
            lines.append("[raw html omitted]")
        print("\n".join(lines))

    return 0 if result.status == 200 else 4


if __name__ == "__main__":
    sys.exit(main())
