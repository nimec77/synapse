# Deploying synapse-telegram to a VPS

This guide covers deploying the `synapse-telegram` bot to a VPS running Ubuntu 22.04 or 24.04. The binary is compiled on a separate Ubuntu build machine and copied to the VPS — no Rust toolchain required on the server.

**Target environment:** 1 CPU, 2 GB RAM, 40 GB disk, Ubuntu 22.04/24.04, 1 IPv4 address.
**LLM provider:** DeepSeek.

---

## 1. Build on the build machine

Install Rust nightly and system dependencies on the **build machine** (Ubuntu):

```bash
# System dependencies
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev libsqlite3-dev

# Rust nightly (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup override set nightly
```

Clone the repository and build the release binary:

```bash
git clone <your-repo-url> synapse
cd synapse
cargo build --release -p synapse-telegram
```

The binary will be at `target/release/synapse-telegram`.

---

## 2. Prepare the VPS

Run the following on the **VPS** as root or with `sudo`:

```bash
# Create a dedicated system user with no login shell
sudo useradd --system --no-create-home --shell /usr/sbin/nologin synapse

# Create required directories
sudo mkdir -p /usr/local/bin          # binary
sudo mkdir -p /etc/synapse            # config and env file
sudo mkdir -p /var/lib/synapse        # SQLite database
sudo mkdir -p /var/log/synapse        # log files

# Set ownership
sudo chown synapse:synapse /var/lib/synapse
sudo chown synapse:synapse /var/log/synapse
sudo chown root:root /etc/synapse
```

---

## 3. Transfer the binary

From the **build machine**, copy the binary to the VPS:

```bash
scp target/release/synapse-telegram user@your-vps-ip:/tmp/synapse-telegram
```

On the **VPS**, move it into place:

```bash
sudo mv /tmp/synapse-telegram /usr/local/bin/synapse-telegram
sudo chmod 755 /usr/local/bin/synapse-telegram
```

---

## 4. Configure

Create `/etc/synapse/config.toml` on the VPS:

```bash
sudo nano /etc/synapse/config.toml
```

Paste the following, filling in your `allowed_users`:

```toml
provider = "deepseek"
model = "deepseek-chat"

[session]
database_url = "sqlite:/var/lib/synapse/sessions.db"

[telegram]
# List your Telegram user IDs here. Get your ID from @userinfobot on Telegram.
# WARNING: an empty list rejects ALL users — the bot will be inaccessible.
allowed_users = [123456789]

[logging]
directory = "/var/log/synapse"
max_files = 7
rotation = "daily"
```

If you use `system_prompt_file`, always specify an **absolute path**:

```toml
system_prompt_file = "/etc/synapse/prompt-system.md"
```

A relative path is resolved from `WorkingDirectory` (`/var/lib/synapse`), not from the config file's location, which is a common source of startup failures.

> **Note:** API keys and the bot token are kept out of this file for security.
> They are provided via the environment file in the next step.

Lock down the config file:

```bash
sudo chmod 600 /etc/synapse/config.toml
sudo chown root:root /etc/synapse/config.toml
```

---

## 5. MCP Tools (optional)

MCP (Model Context Protocol) gives the LLM extra capabilities — file access, web search, code execution, and more — by spawning local child processes called MCP servers. The bot works fine without this step; skip it if you don't need tool use.

Because MCP servers are child processes spawned by the bot, **their binaries must be installed on the VPS**.

### Install Node.js (for npx-based servers)

Most MCP servers are published to npm and run via `npx`. Install Node.js on the VPS:

```bash
sudo apt-get install -y nodejs npm
```

### Create the MCP server config

Create `/etc/synapse/mcp_servers.json`:

```bash
sudo nano /etc/synapse/mcp_servers.json
```

Example configuration with the filesystem server (gives the LLM read/write access to a directory):

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "/usr/bin/npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/var/lib/synapse/files"]
    }
  }
}
```

> **Important:** Use the **absolute path** for `command` (find it with `which npx`). A relative command will fail because the bot runs as the `synapse` system user with a minimal environment.

Set permissions (no secrets in this file, so readable by the synapse user is fine):

```bash
sudo chmod 644 /etc/synapse/mcp_servers.json
sudo chown root:root /etc/synapse/mcp_servers.json
```

If you used the filesystem server example, create the target directory and give the synapse user access:

```bash
sudo mkdir -p /var/lib/synapse/files
sudo chown synapse:synapse /var/lib/synapse/files
```

### Add `[mcp]` to config.toml

Open `/etc/synapse/config.toml` and add:

```toml
[mcp]
config_path = "/etc/synapse/mcp_servers.json"
```

Always use an **absolute path** — a relative path is resolved from `WorkingDirectory` (`/var/lib/synapse`), not from the config file's location.

Restart the bot to pick up the change:

```bash
sudo systemctl restart synapse-telegram
```

### Verify tools are loaded

Check the logs for tool discovery:

```bash
sudo journalctl -u synapse-telegram -n 50
```

You should see lines like:

```
INFO synapse_telegram: MCP connected, N tools available
```

If MCP initialisation fails, the bot logs a warning and **continues running without tools** — MCP failures are non-fatal.

---

## 6. Environment file

Create `/etc/synapse/env` to hold secrets:

```bash
sudo nano /etc/synapse/env
```

Contents:

```
TELEGRAM_BOT_TOKEN=your-bot-token-here
DEEPSEEK_API_KEY=your-deepseek-api-key-here
RUST_LOG=synapse_telegram=info
```

Lock it down:

```bash
sudo chmod 600 /etc/synapse/env
sudo chown root:root /etc/synapse/env
```

> Get a bot token from [@BotFather](https://t.me/BotFather) on Telegram.
> Get your DeepSeek API key from [platform.deepseek.com](https://platform.deepseek.com).

---

## 7. systemd service

Create `/etc/systemd/system/synapse-telegram.service`:

```bash
sudo nano /etc/systemd/system/synapse-telegram.service
```

Contents:

```ini
[Unit]
Description=Synapse Telegram Bot
After=network.target

[Service]
Type=simple
User=synapse
Group=synapse
WorkingDirectory=/var/lib/synapse
EnvironmentFile=/etc/synapse/env
ExecStart=/usr/local/bin/synapse-telegram --config /etc/synapse/config.toml
Restart=on-failure
RestartSec=5

# Security hardening
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable synapse-telegram
sudo systemctl start synapse-telegram
```

---

## 8. Verify

Check that the service started successfully:

```bash
sudo systemctl status synapse-telegram
```

Follow live logs via journald:

```bash
sudo journalctl -u synapse-telegram -f
```

Or tail the rolling log file:

```bash
tail -f /var/log/synapse/synapse-telegram.log
```

A successful start prints:

```
INFO synapse_telegram: Starting Synapse Telegram Bot
INFO synapse_telegram: Restored N Telegram sessions from storage
INFO synapse_telegram: Dispatcher ready — polling for updates
```

Send a message to your bot on Telegram. You should get a response.

The SQLite database is created automatically on first run at `/var/lib/synapse/sessions.db` — no manual migration step is needed.

---

## 9. Firewall

The bot uses **outgoing HTTPS only** (Telegram long-polling + DeepSeek API). No inbound ports need to be opened beyond SSH.

If you are using UFW:

```bash
sudo ufw allow ssh
sudo ufw enable
```

No additional rules are needed for the bot.

---

## 10. Maintenance

### Updating the binary

1. Build a new release on the build machine (`cargo build --release -p synapse-telegram`).
2. Copy it to the VPS:
   ```bash
   scp target/release/synapse-telegram user@your-vps-ip:/tmp/synapse-telegram
   ```
3. On the VPS, replace the binary and restart:
   ```bash
   sudo systemctl stop synapse-telegram
   sudo mv /tmp/synapse-telegram /usr/local/bin/synapse-telegram
   sudo chmod 755 /usr/local/bin/synapse-telegram
   sudo systemctl start synapse-telegram
   ```

### Checking logs

```bash
# Recent journal entries
sudo journalctl -u synapse-telegram --since "1 hour ago"

# Log files (if [logging] is configured)
ls /var/log/synapse/
```

### SQLite backup

```bash
# Copy the database while the bot is running (SQLite WAL mode is safe)
sudo -u synapse sqlite3 /var/lib/synapse/sessions.db ".backup /tmp/sessions-backup.db"
scp user@your-vps-ip:/tmp/sessions-backup.db ./sessions-backup.db
```

To automate, add a cron job or systemd timer that runs the `.backup` command nightly.

---

## 11. Troubleshooting

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| Bot does not respond to any user | `allowed_users` is empty | Add your Telegram user ID to `[telegram]` in config.toml |
| `MissingApiKey` error in logs | `DEEPSEEK_API_KEY` not set | Check `/etc/synapse/env` and `systemctl restart synapse-telegram` |
| `AuthenticationError` | Invalid API key or bot token | Verify credentials in `/etc/synapse/env` |
| `Permission denied` on DB or logs | Wrong directory ownership | Run `sudo chown synapse:synapse /var/lib/synapse /var/log/synapse` |
| `failed to read config file 'etc/...'` | Relative path in `system_prompt_file` | Use an absolute path: `/etc/synapse/prompt-system.md` |
| Service exits immediately | Config file missing or invalid | Run `sudo -u synapse /usr/local/bin/synapse-telegram --config /etc/synapse/config.toml` to see the error directly |
| OOM / service killed | Bot using too much memory | Upgrade VPS RAM or reduce session retention |
| `Config file not found` error | Wrong path to config | Ensure `--config /etc/synapse/config.toml` is in the `ExecStart` line |
| No MCP tools available (no warning in logs) | `[mcp]` missing from config or JSON file not found | Create `/etc/synapse/mcp_servers.json` and add `[mcp] config_path = "/etc/synapse/mcp_servers.json"` to config.toml |
| `MCP initialization failed` warning in logs | Server binary not found or crashed on startup | Install the binary; test manually with `sudo -u synapse /usr/bin/npx -y @modelcontextprotocol/server-filesystem /tmp` |
