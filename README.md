# drift CLI

Command-line client for [Drift](https://rdrift.app). Talks to the live HTTP API — it does not read a local Rails database.

The Rails app lives in [`iuhoay/drift`](https://github.com/iuhoay/drift).

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/iuhoay/drift-cli/main/scripts/install.sh | bash
drift auth login
```

The installer downloads the latest `v*` binary (Darwin arm64 or Linux amd64) into `~/.local/bin`. Do not `cargo install` the repo. Later upgrades: `drift update`.

Sign in through the browser (GitHub, Google, or password). The CLI never asks you to paste a token. After you click **Authorize CLI**, a one-time code is handed back to the local listener and exchanged for an API token stored in `$XDG_CONFIG_HOME/drift/config.toml` (or `~/.config/drift/config.toml`) with mode `0600`.

Non-interactive override: `drift auth login --token <token>` (the flag is hidden from `--help`).

## Commands

| Command | What it does |
| --- | --- |
| `drift auth login` | Open a browser, sign in, store a token (the setup step) |
| `drift auth status` | Show host, masked token, and whether the API accepts it |
| `drift feeds` | List subscribed feeds |
| `drift inbox [--feed ID] [--limit N]` | Unread entries (server default 20, max 50) |
| `drift search <query> [--limit N]` | Search entries |
| `drift show <id>` | Print one entry's `body` from the API |
| `drift update` | Replace this binary with the latest GitHub Release |

There is no mark-read, star, or saved-items command in v1. `drift update` talks to GitHub, not the Drift API, and does not need a login.

## Global flags

| Flag | Meaning |
| --- | --- |
| `--output json\|text` | Default is one line of raw JSON; `text` is the human table |
| `--host <url>` | Override the API host |
| `--token <token>` | Override the bearer token (hidden from `--help`) |

`--output json` prints one raw JSON object (what an agent should parse).

## Environment

Highest wins: flag → env → config file.

| Variable | Meaning |
| --- | --- |
| `DRIFT_HOST` | API base URL (trailing slash stripped) |
| `DRIFT_TOKEN` | Bearer token |

Config file:

```toml
host = "https://rdrift.app"
token = "..."
```

## Agent skill

Source: [`skills/drift-cli`](skills/drift-cli). Agents install skills their own way. `npx skills add iuhoay/drift-cli` finds this one skill.

## Developing

```sh
cargo test
cargo clippy --all-targets -- -D warnings
bash -n scripts/install.sh
cargo install --path .
```

Cut a release after merging to main:

```sh
git tag v0.1.2
git push origin v0.1.2
```

## License

GNU Affero General Public License v3.0. See [LICENSE](LICENSE).
