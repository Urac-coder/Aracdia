# Aracdia — dedicated server

This folder will host the dedicated server configuration and Docker setup for
Aracdia, targeting 50–200 simultaneous players.

Layout (planned):

- `minetest.conf` — server-side engine config (max users, port, world params)
- `docker/` — Dockerfile + compose file to run the server in production
- `scripts/` — backups, world rotation, monitoring helpers
