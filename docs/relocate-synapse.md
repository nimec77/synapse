# Relocating synapse-telegram to a New VPS

This guide covers migrating an existing `synapse-telegram` deployment to a new VPS, with `telegram-mcp` (the [telegram-connector](https://github.com/nimec77/telegram-connector) MCP server) deployed alongside it. The process starts with a clean database on the new server — conversation history is not migrated, only configuration.

**Assumed environment:** Both servers are Ubuntu 22.04/24.04, 1 CPU, 2 GB RAM, 40 GB disk.

---

## 1. Back up config from the old server

SSH into the **old VPS** and collect the config files you'll need on the new one:

```bash
# On the old VPS
mkdir -p /tmp/synapse-backup
sudo cp /etc/synapse/config.toml /tmp/synapse-backup/
sudo cp /etc/synapse/env         /tmp/synapse-backup/
sudo cp /etc/synapse/mcp_servers.json /tmp/synapse-backup/ 2>/dev/null || true

# If you use a system prompt file
sudo cp /etc/synapse/prompt-system.md /tmp/synapse-backup/ 2>/dev/null || true

# telegram-connector session and config (if deployed on old server)
sudo cp /var/lib/synapse/telegram-connector/session.bin /tmp/synapse-backup/ 2>/dev/null || true
sudo cp /etc/telegram-mcp/config.toml /tmp/synapse-backup/telegram-mcp-config.toml 2>/dev/null || true

# Fix ownership so you can scp them
sudo chown "$USER":"$USER" /tmp/synapse-backup/*
```

From your **local machine**, pull the backup:

```bash
scp -r user@old-vps-ip:/tmp/synapse-backup ./synapse-backup
```

> **Database:** The SQLite file at `/var/lib/synapse/sessions.db` is intentionally not copied — the new deployment starts with a clean database.

---

## 2. Prepare the new server

Run the following on the **new VPS** as root or with `sudo`:

```bash
# Dedicated system user — no login shell
sudo useradd --system --no-create-home --shell /usr/sbin/nologin synapse

# Required directories
sudo mkdir -p /usr/local/bin          # binaries
sudo mkdir -p /etc/synapse            # config and env file
sudo mkdir -p /var/lib/synapse        # SQLite database + telegram-connector session
sudo mkdir -p /var/log/synapse        # log files
sudo mkdir -p /etc/telegram-mcp       # telegram-connector config

# Ownership
sudo chown synapse:synapse /var/lib/synapse
sudo chown synapse:synapse /var/log/synapse
sudo chown root:root /etc/synapse
sudo chown root:synapse /etc/telegram-mcp
```

---

## 3. Build both binaries on the build machine

Install Rust nightly and system dependencies on the **build machine** (Ubuntu) if not already present:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev libsqlite3-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup override set nightly
```

Build `synapse-telegram`:

```bash
cd synapse
cargo build --release -p synapse-telegram
# Binary: target/release/synapse-telegram
```

Build `telegram-mcp` (clone the repository alongside your synapse checkout):

```bash
git clone https://github.com/nimec77/telegram-connector
cd telegram-connector
cargo build --release
# Binary: target/release/telegram-mcp
```

---

## 4. Transfer binaries to the new server

From the **build machine**:

```bash
scp target/release/synapse-telegram           user@new-vps-ip:/tmp/synapse-telegram
scp ../telegram-connector/target/release/telegram-mcp user@new-vps-ip:/tmp/telegram-mcp
```

On the **new VPS**, install both:

```bash
sudo mv /tmp/synapse-telegram /usr/local/bin/synapse-telegram
sudo chmod 755 /usr/local/bin/synapse-telegram

sudo mv /tmp/telegram-mcp /usr/local/bin/telegram-mcp
sudo chmod 755 /usr/local/bin/telegram-mcp
```

---

## 5. Transfer config files from the backup

From your **local machine**:

```bash
scp synapse-backup/config.toml        user@new-vps-ip:/tmp/config.toml
scp synapse-backup/env                user@new-vps-ip:/tmp/synapse-env
scp synapse-backup/mcp_servers.json   user@new-vps-ip:/tmp/mcp_servers.json 2>/dev/null || true
scp synapse-backup/prompt-system.md        user@new-vps-ip:/tmp/prompt-system.md 2>/dev/null || true
scp synapse-backup/telegram-mcp-config.toml user@new-vps-ip:/tmp/telegram-mcp-config.toml 2>/dev/null || true
scp synapse-backup/session.bin             user@new-vps-ip:/tmp/session.bin 2>/dev/null || true
```

On the **new VPS**, place the files:

```bash
sudo mv /tmp/config.toml      /etc/synapse/config.toml
sudo mv /tmp/synapse-env      /etc/synapse/env
sudo mv /tmp/mcp_servers.json /etc/synapse/mcp_servers.json 2>/dev/null || true
sudo mv /tmp/prompt-system.md          /etc/synapse/prompt-system.md 2>/dev/null || true
sudo mv /tmp/telegram-mcp-config.toml /etc/telegram-mcp/config.toml 2>/dev/null || true

# Lock down secrets
sudo chmod 640 /etc/synapse/config.toml
sudo chmod 640 /etc/synapse/env
sudo chmod 640 /etc/telegram-mcp/config.toml 2>/dev/null || true
sudo chown root:synapse /etc/synapse/config.toml /etc/synapse/env
sudo chown root:synapse /etc/telegram-mcp/config.toml 2>/dev/null || true
```

---

## 6. Configure telegram-connector

`telegram-mcp` needs its own TOML configuration file with Telegram API credentials (from [my.telegram.org](https://my.telegram.org)) and explicit paths for its session file and logs.

### Add credentials to the env file

Open `/etc/synapse/env` and append the Telegram API credentials:

```bash
sudo nano /etc/synapse/env
```

Add these lines:

```
TELEGRAM_API_ID=your-api-id-here
TELEGRAM_API_HASH=your-api-hash-here
TELEGRAM_PHONE_NUMBER=+your-phone-number-here
```

> **Note:** `TELEGRAM_API_HASH` and `TELEGRAM_PHONE_NUMBER` are only needed for `--setup` (initial authentication). After setup completes, you can remove them from the env file.

### Create the telegram-connector config file

`telegram-mcp` reads a TOML config file, not raw environment variables. The config file uses `${VAR}` syntax to expand the env vars set above.

If a `config.toml` was restored from the old server (Section 5), verify its paths are correct. Otherwise, create a new one:

```bash
sudo nano /etc/telegram-mcp/config.toml
```

Contents:

```toml
[telegram]
api_id = "${TELEGRAM_API_ID}"
api_hash = "${TELEGRAM_API_HASH}"
phone_number = "${TELEGRAM_PHONE_NUMBER}"
session_file = "/var/lib/synapse/telegram-connector/session.bin"

[logging]
level = "info"
file_enabled = true
file_path = "/var/log/synapse/"
max_log_days = 7
```

Set permissions — the file contains `${VAR}` placeholders (not actual secrets), but the `synapse` user needs read access at runtime:

```bash
sudo chmod 640 /etc/telegram-mcp/config.toml
sudo chown root:synapse /etc/telegram-mcp/config.toml
```

### Create the session and log directories

```bash
sudo mkdir -p /var/lib/synapse/telegram-connector
sudo chown synapse:synapse /var/lib/synapse/telegram-connector

# Ensure the log directory exists and is writable (also created in Section 2)
sudo mkdir -p /var/log/synapse
sudo chown synapse:synapse /var/log/synapse
```

> **If migrating an existing session:** Copy the backed-up `session.bin` to skip re-authentication (Section 9):
> ```bash
> sudo cp /tmp/session.bin /var/lib/synapse/telegram-connector/session.bin
> sudo chown synapse:synapse /var/lib/synapse/telegram-connector/session.bin
> ```

`telegram-mcp` is spawned as a child process of `synapse-telegram` and inherits the service environment from the `EnvironmentFile`, so the `${VAR}` references in the TOML will resolve correctly.

---

## 7. Update mcp_servers.json

Open `/etc/synapse/mcp_servers.json` (or create it if it did not exist on the old server):

```bash
sudo nano /etc/synapse/mcp_servers.json
```

Add the `telegram` entry. Use an absolute path for `command` and pass `--config` pointing to the telegram-connector config file:

```json
{
  "mcpServers": {
    "telegram": {
      "command": "/usr/local/bin/telegram-mcp",
      "args": ["--config", "/etc/telegram-mcp/config.toml"]
    }
  }
}
```

If the file already contains other MCP servers (e.g. `filesystem`), add `"telegram"` as an additional key inside `"mcpServers"`.

Set permissions:

```bash
sudo chmod 644 /etc/synapse/mcp_servers.json
sudo chown root:root /etc/synapse/mcp_servers.json
```

---

## 8. Ensure config.toml points to mcp_servers.json

Open `/etc/synapse/config.toml` and confirm (or add) the `[mcp]` section:

```bash
sudo nano /etc/synapse/config.toml
```

Required block:

```toml
[mcp]
config_path = "/etc/synapse/mcp_servers.json"
```

Always use an **absolute path** — a relative path is resolved from `WorkingDirectory` (`/var/lib/synapse`), not from the config file's location.

---

## 9. Authenticate telegram-connector interactively

`telegram-mcp --setup` must run as the `synapse` user so it writes the session file to the right place with the right ownership. The env vars must be available for `${VAR}` expansion in the config file to work.

SSH into the **new VPS** and run:

```bash
sudo -u synapse bash -c 'set -a && . /etc/synapse/env && set +a && \
  /usr/local/bin/telegram-mcp \
  --setup --config /etc/telegram-mcp/config.toml'
```

The setup wizard will prompt for the one-time code sent to your Telegram client (the phone number comes from the config file). Once complete, the session file is written to `/var/lib/synapse/telegram-connector/session.bin`. You will not need to repeat this step unless the session expires or is revoked.

> **Tip:** If you restored a `session.bin` from the old server in Section 6, you can skip this step entirely.

---

## 10. Set up the systemd service

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

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable synapse-telegram
sudo systemctl start synapse-telegram
```

---

## 11. Verify

Check that the service started:

```bash
sudo systemctl status synapse-telegram
```

Follow live logs:

```bash
sudo journalctl -u synapse-telegram -f
```

A successful start with MCP connected prints:

```
INFO synapse_telegram: Starting Synapse Telegram Bot
INFO synapse_telegram: Restored N Telegram sessions from storage
INFO synapse_telegram: MCP connected, N tools available
INFO synapse_telegram: Dispatcher ready — polling for updates
```

Send a message to your bot on Telegram to confirm end-to-end functionality. The SQLite database is created automatically on first message.

---

## 12. Decommission the old server

Once you have confirmed the new deployment is working:

1. Stop the service on the old VPS:
   ```bash
   sudo systemctl stop synapse-telegram
   sudo systemctl disable synapse-telegram
   ```
2. (Optional) Take a final config backup from the old server before terminating it.
3. Remove or destroy the old VPS according to your provider's procedure.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| Bot does not respond | `allowed_users` list wrong after migration | Verify your Telegram user ID in `[telegram]` in config.toml |
| `MissingApiKey` in logs | Env var not set | Check `/etc/synapse/env` and `sudo systemctl restart synapse-telegram` |
| `MCP initialization failed` | `telegram-mcp` not authenticated | Re-run setup: `sudo -u synapse bash -c 'set -a && . /etc/synapse/env && set +a && /usr/local/bin/telegram-mcp --setup --config /etc/telegram-mcp/config.toml'` |
| `Permission denied` on session file | Wrong ownership | `sudo chown -R synapse:synapse /var/lib/synapse/telegram-connector` |
| `Permission denied` on config file | Config not readable by synapse | `sudo chmod 640 /etc/telegram-mcp/config.toml && sudo chown root:synapse /etc/telegram-mcp/config.toml` |
| `telegram-mcp` config not found | Missing `--config` flag or wrong path | Ensure mcp_servers.json passes `"--config", "/etc/telegram-mcp/config.toml"` in the args array |
| `api_id` parse error or empty value | Env vars not loaded for `${VAR}` expansion | Check that `EnvironmentFile=/etc/synapse/env` is set in the systemd unit and contains `TELEGRAM_API_ID` |
| `No MCP tools available` | `[mcp]` section missing or wrong path | Ensure `config_path = "/etc/synapse/mcp_servers.json"` is set in config.toml |
| `Config file not found` error | Wrong path to config | Verify `--config /etc/synapse/config.toml` is in the `ExecStart` line |
| Service exits immediately | Config invalid or binary incompatible | Run `sudo -u synapse /usr/local/bin/synapse-telegram --config /etc/synapse/config.toml` directly to see the error |
| `failed to read config file 'etc/...'` | Relative `system_prompt_file` path | Use an absolute path: `/etc/synapse/prompt-system.md` |
