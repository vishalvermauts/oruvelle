# echo-ide-mcp

Let your IDE talk to your Amazon Echo. An MCP server that lets any MCP-capable IDE or AI assistant make an Echo speak, play music, set volume, control smart lights (colour, brightness, blinking), and run Alexa routines, all from a tool call. A `notify` command also lets an IDE hook signal when a task finishes.

> **Unofficial.** This uses Amazon's private Alexa web API through [`alexa-remote2`](https://www.npmjs.com/package/alexa-remote2). Amazon does not support it, it can break without notice, and it may conflict with Amazon's terms. Use it only on your own account.

## Features

- **Speak:** make any Echo say text.
- **Any command:** send anything Alexa understands ("play Spotify", "set a 10-minute timer", "add milk to my shopping list").
- **Volume:** set an Echo's speaker volume, 0–100.
- **Lights:** list smart lights, turn them on or off, set brightness, set colour, or blink a light or room group in a colour.
- **Routines:** list and run Alexa routines by name, including partial names.
- **Notify:** one call for IDE events. It sets or blinks a light and speaks a message.
- **CLI:** `notify.js` works from any hook or script, without MCP.

## How it works

```
IDE or AI assistant (any MCP client)
        │  MCP (stdio)
        ▼
  server.js  ── tool definitions (zod schemas)
        │
        ▼
  alexa.js   ── one shared logged-in client, helpers
        │  alexa-remote2 (Amazon's private web API)
        ▼
  Amazon cloud ──► your Echo devices and smart lights

notify.js ──► alexa.js   (used by IDE hooks, no MCP needed)
```

- **Login** happens once, in your browser, on Amazon's real sign-in page. The local proxy at `http://localhost:3456` captures the session, which is saved to `auth.json`.
- **Later runs** reuse `auth.json`. The session is refreshed automatically. If it expires, delete `auth.json` and log in again.

## Requirements

- Node.js 20.11 or newer (the code uses `import.meta.dirname`)
- An Amazon account with at least one Echo device
- Smart lights added through the Alexa app (optional, for light tools)

## Setup

```bash
git clone https://github.com/<you>/echo-ide-mcp.git
cd echo-ide-mcp
npm install
```

### 1. Log in once

Start any tool call (or run `node notify.js "" green "Lamp 1"`, which only sets a colour). The first call opens the login proxy. Visit **http://localhost:3456/**, sign in with your Amazon account, and wait for `LOGGED IN, saved auth.json` in the terminal. Do not share `auth.json`.

If your account is not on amazon.com, set the region:

```bash
AMAZON_PAGE=amazon.co.uk node notify.js "hello"
```

### 2. Choose your Echo

The default device name is set in `alexa.js`. Override it with an environment variable:

```bash
ALEXA_DEVICE="Kitchen Echo" node notify.js "hello"
```

### 3. Connect your IDE

Add this server to your MCP client's configuration file. Cursor, VS Code and other MCP clients use the same `command` and `args`:

```json
{
  "mcpServers": {
    "alexa": {
      "command": "node",
      "args": ["C:/path/to/echo-ide-mcp/server.js"]
    }
  }
}
```

Restart the client fully so the tools load.


### 4. Optional: signal from IDE hooks

To flash a light when a response finishes, add a `Stop` hook to your IDE's hook settings (hook systems that run a command at the end of a response):

```json
"hooks": {
  "Stop": [
    { "hooks": [ { "type": "command", "command": "node C:/path/to/echo-ide-mcp/notify.js \"Done\" green \"Lamp 1\"" } ] }
  ]
}
```

`Stop` runs after every response, so keep the colour or target light simple. Add `blink` as the fourth argument to flash the light instead.

## Tools

| Tool | What it does | Main inputs |
|---|---|---|
| `alexa_devices` | List Echo devices on the account | none |
| `alexa_speak` | Make an Echo say text | `text`, optional `device` |
| `alexa_command` | Send any command as if spoken | `text`, optional `device` |
| `alexa_volume` | Set speaker volume | `level` (0–100), optional `device` |
| `alexa_lights_list` | List smart lights | none |
| `alexa_light` | Turn light/room on or off, set brightness or colour | `target`, `action` (`turnOn`, `turnOff`, `setBrightness`, `setColor`), `brightness`, `color` |
| `alexa_blink` | Blink a light or room in a colour, then leave it on | `target`, `color`, `times` (default 3) |
| `alexa_routines_list` | List routines by name | none |
| `alexa_routine` | Run a routine by name | `name`, optional `device` |
| `alexa_notify` | Set or blink lights and speak a message | `target`, `color`, `blink`, `message`, optional `device` |

Colours are Alexa colour names in lower case, such as `red`, `blue`, `green`, `warm_white`. Volume is 0–100, and brightness is 0–100.

## Command-line use

```bash
node notify.js "Build finished" green "Lamp 1"          # set colour and speak
node notify.js "" red "Bedroom" blink                   # blink only
```

Arguments are: message, colour (default `green`), light or room (default `Lamp 1`), and `blink` if you want it to flash. Exit code 0 means success.

## Project layout

| File | Purpose |
|---|---|
| `server.js` | MCP server and tool definitions |
| `alexa.js` | Shared login, device lookup, light, speech and routine helpers |
| `notify.js` | Command-line wrapper for hooks and scripts |
| `.mcp.json` | Example MCP config for this folder |
| `auth.json` | Saved login session. **Created on first login. Never commit it.** (gitignored) |

## Security

- `auth.json` contains your logged-in Amazon session. Anyone with it can act on your account. It is gitignored. Do not paste it into issues or chats.
- The login proxy listens on `localhost` only, and only during login.
- Your Amazon password is entered on Amazon's own page. This project never sees it.
- Logs can include tokens. Do not commit log files.

## Limitations

- The unofficial API can change or break at any time.
- Light names must match the Alexa app exactly, or use a room group.
- Responses report success from Amazon, not what happened in the room. Check the device to be sure.
- `alexa_notify` and `alexa_blink` need lights that support colour. Plain on/off bulbs will only turn on or off.
- Amazon may rate-limit or flag frequent automated requests. Avoid hooks that fire many times a minute.

## Status

| Feature | Status |
|---|---|
| Login and session reuse | Tested |
| List Echo devices and lights | Tested |
| Speak and volume | Tested |
| Set light colour | Tested (Amazon returned success; check the bulb) |
| Blink, brightness, routines, MCP tool calls | Not yet tested end-to-end |

## Credits

- [`alexa-remote2`](https://github.com/Apollon77/alexa-remote) by Apollon77, which handles Amazon's API.
- Ideas drawn from [chrischall/alexa-mcp](https://github.com/chrischall/alexa-mcp), [m0nkmaster/alexa-mcp](https://github.com/m0nkmaster/alexa-mcp).

## Licence

Not yet chosen. Until a licence is added, all rights are reserved by the author.

