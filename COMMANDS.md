# Syncara Commands

## syncara init

Create a default configuration file.

```sh
syncara init                          # creates syncara.yml
syncara init my-config.yml            # custom filename
syncara init --example brain          # from example config
syncara init --example sticky-sessions
syncara init --example websocket
```

**Use case:** Starting a new project — generates a working config so you don't write YAML from scratch.

---

## syncara validate

Parse and validate the configuration file.

```sh
syncara validate                      # validates syncara.yml
syncara -c my-config.yml validate     # validates a custom config path
```

**Use case:** Before starting the proxy, check that your config has no syntax errors, missing fields, or invalid strategies.

---

## syncara start

Start the Syncara proxy server.

```sh
syncara start                         # start with syncara.yml
syncara -c my-config.yml start        # start with custom config
syncara start --backend localhost:3000 # zero-config mode (port 8080)
syncara start --backend localhost:3000 --port 9000
```

**Use case:** Run the proxy. In zero-config mode (`--backend`), generates a config on the fly — useful for quick dev/testing.

---

## syncara status

Show live upstream/pool status from a running Syncara process.

```sh
syncara status                        # connects to http://127.0.0.1:9090
syncara status --admin http://192.168.1.10:9090
```

**Use case:** Check which upstreams are healthy, active connection counts, and latency without opening the dashboard.

---

## syncara stop

Stop a running Syncara process.

```sh
syncara stop                          # sends SIGTERM
syncara stop --signal INT             # send SIGINT instead
syncara stop --signal HUP             # send SIGHUP (same as reload)
```

**Use case:** Gracefully shut down the proxy from the command line without needing to find the PID.

---

## syncara reload

Validate config and send SIGHUP to reload without downtime.

```sh
syncara reload                        # validates config, then sends SIGHUP
```

**Use case:** Apply config changes (new routes, pools, upstreams) while the proxy is running — zero downtime.

---

## syncara doctor

Run diagnostic checks on configuration and process.

```sh
syncara doctor
```

**Use case:** Quick health check of the whole system — validates config, checks if process is running, probes listener ports.

---

## syncara tune

System tuning recommendations and diagnostics.

```sh
syncara tune
```

**Use case:** Before deploying to production — checks CPU cores, memory, file descriptor limits, and suggests optimal settings.

---

## syncara update

Update Syncara to the latest version.

```sh
syncara update                        # latest version
syncara update --version 0.1.0        # specific version
```

**Use case:** Upgrade to a new release — downloads the binary from GitHub and replaces itself.

---

## syncara uninstall

Uninstall Syncara and remove related files.

```sh
syncara uninstall                     # prompts for confirmation
syncara uninstall --force             # skip confirmation
```

**Use case:** Remove Syncara from your system — deletes the binary and PID file.
