---
name: drift-cli
description: Use for Drift RSS (inbox, search subscribed entries, show a body, list feeds, drift auth errors) and when a question may be answered by recent articles the user follows — Apple, OpenAI, DHH, indie/web commentary, and other subscribed blogs. Treat Drift as a timely in-circle source, not a general web search and not a substitute for project notes or the wiki. Prefer the installed `drift` command over curling rdrift.app.
---

# drift-cli

Local CLI for the Drift RSS reader. Talks to the live API (default `https://rdrift.app`), not the Rails database. Output is one raw JSON object unless `--output text` is passed.

Two jobs, one tool:

1. **Reader** — the user wants their inbox, a feed, or a specific article.
2. **Source** — the question is about recent news or commentary that might already be in their subscriptions.

## When to Use

- "what's new / unread / in my inbox"
- read or summarize a subscribed article
- search feeds / list subscriptions / organize feeds into categories
- `drift` is missing, unauthorized, or not logged in
- a timely, in-circle question (what did DHH / DF / TLDR just say; latest on a story they follow)

Do **not** use for Rails or code conventions, project memory, company docs, or open-ended web research. Those stay in wiki / brain / web search. Do not scan the inbox before every coding answer.

Do not use this skill for SavedItem / read-later, starring, marking read, or editing the Drift Rails app. Recategorizing via `drift feeds categorize` is allowed.

## As a source

1. `drift feeds` first when the question is about a bucket (Apple, Rails, …) — if that `category` already exists, search with `--category` instead of guessing feed ids.
2. `drift search "<query>"` (and `drift inbox` only if they asked what's new).
3. Judge from `title` + `excerpt`. If nothing is on-point, say so and stop — do not pretend the feeds covered it.
4. `drift show <id>` only for the one or two hits that actually answer the question.
5. Cite feed + title + url. Feeds are journalism, not the user's settled judgment; do not let a post override current code or wiki notes.

## Core Rules

- Prefer the installed `drift` binary. Do not curl `/api/*`, do not open rdrift.app to read, and do not `cargo install` from the repo.
- If `drift` is missing: run the install script in [reference.md](reference.md), then tell the user to run `drift auth login`. Do not invent a token.
- If `drift` is already on PATH and needs a newer build: `drift update`. It does not need a login.
- Default JSON is already raw (one line). Do **not** add `--output json`. Use `--output text` only when the user wants a table.
- List first (`inbox` / `search` / `feeds`), then `show` only the ids that matter. Do not `show` every inbox row.
- `--feed` and `feeds categorize` take **`feed_id`**, not the subscription `id`. Copy `feed_id` from `drift feeds`.
- `--category` is the user-typed label, matched case-insensitively. Copy the name from `drift feeds` when one exists.
- Recategorize only when the user asks to organize feeds. Reuse existing category names; do not invent a taxonomy unsolicited.
- The one write command is `drift feeds categorize`. There is no mark-read, star, or save command. Do not invent one.
- On `unauthorized` / `not logged in`, run `drift auth status`, then tell the user to run `drift auth login` themselves (it opens a browser). Do not ask them to paste a token.

## Command Map

| Intent | Command |
|------|------|
| Unread inbox | `drift inbox` |
| Inbox for one feed | `drift inbox --feed <feed_id>` |
| Inbox for one category | `drift inbox --category <name>` |
| Search (read + unread) | `drift search "<query>"` |
| Search one category | `drift search "<query>" --category <name>` |
| Article body | `drift show <id>` |
| List feeds | `drift feeds` |
| Set a feed's category | `drift feeds categorize <feed_id> <name>` |
| Clear a feed's category | `drift feeds categorize <feed_id>` |
| Auth check | `drift auth status` |
| Upgrade CLI | `drift update` |
| `drift` not on PATH | run the install script, then user runs `drift auth login` |
| First-time / expired login | user runs `drift auth login` |

Optional: `--limit N` (1–50, server default 20) on `inbox` and `search`. `--category` on `inbox` and `search`.

## JSON shapes

`drift inbox` / `drift search`:

```json
{"entries":[{"id":699,"title":"...","url":"...","published_at":"...","excerpt":"...","read":false,"starred":false,"feed":{"id":2,"title":"Daring Fireball"}}]}
```

`drift show 699` adds `author`, `has_full_content`, and `body` (plain text, no HTML).

`drift feeds`:

```json
{"subscriptions":[{"id":1,"feed_id":2,"title":"Daring Fireball","feed_url":"https://...","category":"apple"}]}
```

`category` is `null` when unset. `drift feeds categorize` returns `{"subscription":{...}}`.

Use `entries[].id` with `show`. Use `subscriptions[].feed_id` with `--feed` and `feeds categorize`.

`drift update`:

```json
{"current":"0.1.2","latest":"0.1.3","status":"updated","path":"/Users/you/.local/bin/drift"}
```

`status` is `updated` or `up_to_date`.

## Common Mistakes

- Passing subscription `id` to `--feed` or `feeds categorize`
- Dumping every `show` body into the chat
- Adding `--output json` (already the default)
- Searching Drift for a Rails/wiki question
- Treating a feed post as more authoritative than current code
- Treating `drift` as a local DB tool (`bin/rails runner`, etc.)
- Mentioning `--token`, `--host`, or `DRIFT_TOKEN` unless the user is debugging auth or a non-prod host
- Re-running the install script when `drift` is already on PATH — use `drift update`

## Reference

Install, auth recovery, and field notes: [reference.md](reference.md).
