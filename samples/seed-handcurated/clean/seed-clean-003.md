# logfan

A small CLI for tailing structured logs across multiple files.

## Usage

```bash
logfan --filter "level=error" /var/log/app/*.log
```

The `--filter` flag accepts any boolean expression over JSON keys in the log line. Lines that don't parse as JSON are passed through unchanged unless `--strict-json` is set.

## Installation

```bash
cargo install logfan
```

## Configuration

`logfan` reads `~/.config/logfan/config.toml` if present. See [docs/config.md](docs/config.md) for the full schema.

## License

MIT
