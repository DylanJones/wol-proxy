# Agents Guide

This repository includes small utilities to help automate tasks by external agents and scripts.

## .github/tools/web_search.py

A minimal web-page fetcher that mirrors a typical "web search" or "fetch webpage" tool. It’s dependency-light and returns normalized JSON for easy consumption.

- Fetch a URL
- Extracts main readable text (rough heuristic) and the page title
- Optionally include raw HTML
- Optional query filter to keep sentences that contain a keyword
- Bounded output via `--max-chars`

### Interface

Inputs (flags):
- `--url <string>`: Required. HTTP/HTTPS URL to fetch
- `--query <string>`: Optional. If provided, text is filtered to sentences containing the query (case-insensitive)
- `--raw`: Optional. Include raw HTML in the output
- `--format {json|text}`: Output format. Default `json`
- `--max-chars <int>`: Trim text/html to at most N characters (default 20000)

Outputs:
- JSON object (when `--format json`) with fields:
  - `url` (string)
  - `status` (int HTTP status)
  - `title` (string|null)
  - `text` (string|null) — extracted readable text
  - `html` (string|null) — raw HTML (included only with `--raw`)
- Plain text (when `--format text`), suitable for quick inspection

Exit codes:
- `0` success (HTTP 200)
- `2` bad usage / missing dependencies
- `3` network error
- `4` non-200 HTTP status
- `5` parse error (not currently raised; parse issues are logged but non-fatal)

### Examples

Fetch as JSON:

```fish
python .github/tools/web_search.py --url https://example.com --format json
```

Fetch as text and filter by a keyword:

```fish
python .github/tools/web_search.py --url https://www.python.org --query "release" --format text --max-chars 4000
```

Include raw HTML in JSON:

```fish
python .github/tools/web_search.py --url https://example.com --raw --format json --max-chars 5000
```

### Contract for other agents

- Input: flags described above; at minimum, `--url` must be provided.
- Output: JSON with fields `url`, `status`, `title`, `text`, `html`.
- Errors: non-zero exit codes and stderr messages for network/HTTP/usage issues.
- Rate limiting and robots.txt: This helper does not implement throttling or robots.txt checks. Agents should be respectful and add delays or checks as appropriate for their workloads.

### Implementation notes

- Uses `requests` for HTTP with a reasonable default User-Agent and 20s timeout.
- Parses HTML with BeautifulSoup and strips typical non-content tags to derive readable text. This is a simple heuristic, not a full readability algorithm.
- When `--query` is supplied, the text output is reduced to sentences containing the query to help agents pull relevant snippets.
