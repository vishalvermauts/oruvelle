# oruvelle

Control your Amazon Echo and smart lights from any MCP-capable IDE, CLI or AI assistant. Speak, run any Alexa command, set volume, change light colour, blink lights, and run routines, all as tool calls.

The server is a single small Rust program (about 5 MB of RAM while running). Node.js is only used once in a while, to log in and refresh the session.

> **Unofficial.** This uses Amazon's private Alexa web API through [`alexa-remote2`](https://www.npmjs.com/package/alexa-remote2). Amazon does not support it, it can break without notice, and it may conflict with Amazon's terms. Use it only on your own account.

## Contents

- [Features](#features)
- [How it works](#how-it-works)
- [Requirements](#requirements)
- [Quick install](#quick-install)
- [Install with an AI assistant](#install-with-an-ai-assistant)
- [Install step by step](#install-step-by-step)
- [Connect your IDE or CLI](#connect-your-ide-or-cli)
- [Tools](#tools)
- [Use from scripts and hooks](#use-from-scripts-and-hooks)
- [Session and login](#session-and-login)
- [Project layout](#project-layout)
- [Security](#security)
- [Limitations](#limitations)
- [Status](#status)

## Features

- **Speak:** make any Echo say text.
- **Any command:** send anything Alexa understands, such as "play Spotify", "set a 10-minute timer", or "add milk to my shopping list".
- **Volume:** set an Echo's speaker volume from 0 to 100.
- **Lights:** list smart lights, turn them on or off, set brightness, set colour, or blink a light or room.
- **Routines:** list and run Alexa routines by name, including partial names.
- **Notify:** one call for IDE events. It sets or blinks a light and speaks a message.
- **Shared login:** log in once. Every IDE on the machine uses the same session.

## How it works

```
IDE, CLI or AI assistant
        │  MCP over stdio
        ▼
  oruvelle.exe (Rust, about 5 MB)
        │  reads session.json (cookie, devices, lights, routines)
        ▼
  Amazon Alexa web API  ──►  your Echo devices and smart lights

  session.js (Node, run now and then)
        │  logs in once, refreshes the session, writes session.json
        ▼
  Amazon sign-in  (via http://localhost:3456)
```

- **Login** happens in your browser, on Amazon's own sign-in page. Your password never reaches this project.
- **Session** is saved in `auth.json` and `session.json`. Both are gitignored.
- **Sharing:** every IDE starts its own copy of `oruvelle.exe`, and each copy reads the same `session.json`. You don't log in again for each IDE.

## Requirements

- **Rust and Cargo**, to build the server.
- **Node.js 20.11 or newer**, to log in and refresh the session.
- An **Amazon account** with at least one Echo device.
- Optional: smart lights added through the Alexa app, for the light tools.

## Quick install

If you just want the short version:

```bash
git clone https://github.com/vishalvermauts/oruvelle.git
cd oruvelle
npm install
cd rust && cargo build --release && cd ..
node session.js          # sign in at http://localhost:3456 when prompted
```

Then add the server to your IDE (see [Connect your IDE or CLI](#connect-your-ide-or-cli)).

## Install with an AI assistant

Paste this into any AI coding assistant that can run shell commands in a terminal. It will install the project and walk you through the one sign-in step.

````text
Install the oruvelle Alexa MCP server on this computer and register it with my IDE.

Repository: https://github.com/vishalvermauts/oruvelle.git

Do these steps in order and stop to ask me if something is missing:

1. Check that Git, Node.js (20.11 or newer), npm and Rust (cargo) are installed. If one is missing, tell me how to install it and wait.
2. Clone the repository into a folder called "oruvelle" in my home directory. If the folder already exists, use it and run "git pull" instead.
3. In that folder run: npm install
4. Build the Rust server: cd rust && cargo build --release
   The executable is rust/target/release/oruvelle.exe (or "oruvelle" on macOS and Linux). Note its full absolute path.
5. Log in. Run: node session.js
   It prints a sign-in address (http://localhost:3456/). Ask me to open it in my browser and sign in with my Amazon account. Wait until the output says "LOGGED IN" and "session.json written". Do not try to sign in for me, and never ask me for my password.
6. Test it: run the server with a list-lights request and confirm it returns my light names. For example, pipe two JSON-RPC lines into the executable: an "initialize" request, then a "tools/call" request for "alexa_lights_list".
7. Register the server in my IDE. Use this entry, with the full path from step 4 and no arguments:
   {"mcpServers":{"alexa":{"command":"<full path to oruvelle.exe>","args":[]}}}
   Find the correct config file for my IDE, add the entry without removing my existing servers, and tell me the file you changed. For VS Code, use the "servers" key in .vscode/mcp.json instead.
8. Tell me to restart the IDE, then confirm the tools "alexa_lights_list" and "alexa_speak" are available.

Rules: never print or share the contents of auth.json or session.json. Do not commit them to git. Do not change any Amazon settings. Report what you did and anything that failed.
````

## Install step by step

### 1. Build the server

```bash
cd rust
cargo build --release
```

This creates `rust/target/release/oruvelle.exe` on Windows, or `rust/target/release/oruvelle` on macOS and Linux.

### 2. Log in and save the session

```bash
npm install
node session.js
```

The first run opens a sign-in proxy at **http://localhost:3456/**. Sign in with your Amazon account. When you see `LOGGED IN` and `session.json written`, you're done.

If your Amazon account is not on amazon.com, set the region before running:

```bash
AMAZON_PAGE=amazon.co.uk node session.js
```

The library also detects your region from the account. If it finds `amazon.com.au`, it uses that automatically.

### 3. Choose the default Echo (optional)

The main Echo is the default for every tool. To change it, set `ALEXA_DEVICE` before the command that starts the server, or before a `node session.js` run. Names are the ones shown by `alexa_devices`.

```bash
ALEXA_DEVICE="Kitchen Echo"
```

## Connect your IDE or CLI

The server speaks MCP over standard input and output and needs no arguments. Register the executable path in your client's MCP settings:

```json
{
  "mcpServers": {
    "alexa": {
      "command": "C:/path/to/oruvelle/rust/target/release/oruvelle.exe",
      "args": []
    }
  }
}
```

Where that goes depends on the client:

| Client | Config key | Where to look |
|---|---|---|
| Cursor | `mcpServers` | Cursor's MCP settings |
| VS Code | `servers` in `.vscode/mcp.json` | VS Code's MCP documentation |
| Other MCP clients | `mcpServers` | The client's MCP settings |

Check your client's documentation for the exact file location and key name. Restart the client after you edit the file, so it loads the server.

To check it works, ask your assistant to list your lights. You should see your light names.

## Tools

| Tool | What it does | Main inputs |
|---|---|---|
| `alexa_devices` | List Echo devices on the account | none |
| `alexa_speak` | Make an Echo say text | `text`, optional `device` |
| `alexa_command` | Send any command as if spoken | `text`, optional `device` |
| `alexa_volume` | Set speaker volume | `level` (0–100), optional `device` |
| `alexa_lights_list` | List smart lights | none |
| `alexa_light` | Turn a light or room on or off, set brightness or colour | `target`, `action` (`turnOn`, `turnOff`, `setBrightness`, `setColor`), `brightness`, `color` |
| `alexa_blink` | Blink a light or room in a colour, then leave it on | `target`, `color`, `times` (default 3) |
| `alexa_routines_list` | List routines by name | none |
| `alexa_routine` | Run a routine by name | `name`, optional `device` |
| `alexa_notify` | Set or blink lights and speak a message | `target`, `color`, `blink`, `message`, optional `device` |

Colours are Alexa colour names in lower case, such as `red`, `blue`, `green` and `warm_white`. Volume and brightness are 0 to 100. A light target can be a light name or a room group.

## Use from scripts and hooks

The same executable has a `notify` mode, so scripts and IDE hooks can use it without MCP:

```bash
oruvelle notify "Build finished" green "Lamp 1"     # set colour and speak
oruvelle notify "" red "Bedroom" blink               # blink only, no speech
```

Arguments, in order: message (empty for none), colour (default `green`), light or room (default `Lamp 1`), and a fourth argument `blink` to flash. It exits with code 0 on success.

To flash a light when a response finishes, add a `Stop` hook to your IDE's hook settings. Use the executable path, and keep the command short, because it runs after every response:

```json
"hooks": {
  "Stop": [
    { "hooks": [ { "type": "command", "command": "C:/path/to/oruvelle/rust/target/release/oruvelle.exe notify \"Done\" green \"Lamp 1\"" } ] }
  ]
}
```

## Session and login

- **First login:** `node session.js`, then sign in at http://localhost:3456.
- **Shared:** all IDEs use the same `session.json`, so one login covers them all.
- **Expiry:** when the server reports `session expired`, run `node session.js` again. The IDEs use the new session on their next call, with no restart.
- **How long a session lasts:** Amazon publishes no official figure. Community libraries report the cookie lasts about 14 days and recommend refreshing it every 4 to 13 days. Run `node session.js` at least that often, or schedule it weekly.
- **Do not** run `session.js` on several machines or folders at once. A new login can invalidate older saved sessions.

## Project layout

| File | Purpose |
|---|---|
| `rust/` | The MCP server and `notify` command. The recommended build. |
| `session.js` | Logs in and writes `session.json` for the server. |
| `alexa.js` | Shared login and Alexa helpers used by `session.js` and the Node scripts. |
| `notify.js` | Older Node version of `notify`. Works, but uses about 60 MB per call. Prefer the Rust `notify`. |
| `login.js` | Older login helper. `session.js` replaces it. |
| `.mcp.json` | Example MCP config for this folder. |
| `auth.json`, `session.json` | Your saved login and session. Created on first login. Gitignored. |

## Security

- `auth.json` and `session.json` contain your logged-in Amazon session. Anyone with them can act on your account. Both are gitignored. Do not paste them into issues, chats or commits.
- The sign-in proxy listens on `localhost` only, and only while you are logging in.
- Your Amazon password is entered on Amazon's own page. This project never sees it.
- Do not commit log files, since they can contain tokens.

## Limitations

- The API is unofficial and can change or break without notice.
- Light names must match the Alexa app exactly, or you can use a room group.
- Amazon reports success when it accepts a command, not when the light has changed. Check the device to be sure.
- Colour and blink need colour-capable lights. Plain on/off bulbs only turn on and off.
- Amazon may rate-limit or flag frequent automated requests. Avoid hooks that fire many times a minute.
- Session expiry is manual for now. Run `node session.js` on a schedule to avoid surprises.

## Status

| Feature | Status |
|---|---|
| Rust server, MCP connection, lights and routines listed | Tested through MCP |
| Set light colour through MCP | Tested (Amazon returned `SUCCESS`) |
| Rust `notify`: colour change and speech | Tested |
| Shared login across IDEs | Tested (a fresh install used the same session) |
| Memory use | About 5 MB while running (Node server was about 62 MB) |
| Speak, volume, text commands, blink, routine run through MCP | Not yet tested through MCP |
| Automatic session refresh | Not set up |

## Credits

- [`alexa-remote2`](https://github.com/Apollon77/alexa-remote) by Apollon77, which handles Amazon's API.
- Ideas drawn from [chrischall/alexa-mcp](https://github.com/chrischall/alexa-mcp) and [m0nkmaster/alexa-mcp](https://github.com/m0nkmaster/alexa-mcp).

## Licence

Not yet chosen. Until a licence is added, all rights are reserved by the author.
