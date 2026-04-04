# agent-browser-stealth

Stealth fork of [agent-browser](https://github.com/vercel-labs/agent-browser) — connects to your real Chrome, shares your login sessions, and is undetectable by anti-bot systems.

For basic usage, commands, and API reference, see the [upstream documentation](https://github.com/vercel-labs/agent-browser).

## Why this fork?

**agent-browser** launches a fresh browser with an empty profile. You need to log in again, and websites can detect it's automated.

**agent-browser-stealth** connects to your existing Chrome. Your cookies, sessions, and browser fingerprint are all real — because it IS your real browser.

| | agent-browser | agent-browser-stealth |
|---|---|---|
| Browser | Launches new Chrome | Connects to your Chrome |
| Login state | Empty, need to re-login | Your existing sessions |
| Fingerprint | Automation markers present | Your real fingerprint |
| User collaboration | Separate window | Same window, take over anytime |
| CAPTCHA | Agent stuck | You solve it, agent continues |

## Install

```bash
npm install -g agent-browser-stealth
```

## Setup (one time)

Enable Chrome DevTools Protocol in your Chrome:

1. Open `chrome://inspect/#remote-debugging` in Chrome
2. Toggle the switch on

That's it. This setting persists across Chrome restarts.

## Usage

```bash
# Connect to your Chrome and navigate
agent-browser open https://example.com

# Everything works through your logged-in browser
agent-browser click "Post"
agent-browser fill "Title" "Hello World"
agent-browser screenshot ./page.png
```

The agent operates in your Chrome — you'll see tabs opening, pages loading, clicks happening in real time. You can take over at any point (e.g. solve a CAPTCHA), then let the agent continue.

### Standalone mode

If you need a separate browser (CI, testing, etc.):

```bash
agent-browser --launch open https://example.com
```

In CI environments, standalone mode is used automatically.

## Anti-detection

When connected to your real Chrome, we inject **zero** JavaScript patches. Your browser's fingerprint is completely genuine.

The only thing we do is call `Emulation.setAutomationOverride` via CDP to set `navigator.webdriver = false` at the native Chrome level — undetectable by lie-detection systems like CreepJS.

**Test results (connected to real Chrome):**

| Test site | Result |
|---|---|
| [CreepJS](https://abrahamjuliot.github.io/creepjs/) | 0% stealth, 0% headless |
| [bot.sannysoft.com](https://bot.sannysoft.com) | All green |
| [Cloudflare Turnstile](https://nowsecure.nl) | Passed |

When using `--launch` mode (standalone browser), a full suite of 32 stealth patches is applied for headless Chrome.

## Differences from upstream

Based on [agent-browser v0.24.0](https://github.com/vercel-labs/agent-browser). Changes:

- **Auto-connect is default** — `agent-browser open <url>` connects to your Chrome instead of launching a new one
- **CDP-native stealth** — `Emulation.setAutomationOverride` instead of JS patches
- **Dual stealth mode** — zero patches for real Chrome, full patches for `--launch` mode
- **`--launch` / `--new` flag** — explicitly start a standalone browser
- **CI auto-detection** — standalone mode when `CI` env var is set

All upstream features (commands, snapshots, screenshots, recordings, tabs, sessions, etc.) work the same. See the [upstream repo](https://github.com/vercel-labs/agent-browser) for full documentation.

## License

Apache-2.0 (same as upstream)
