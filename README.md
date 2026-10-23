# veoveo-mcp-console

Console is the Hub MCP server: the MCP provider of the Hub's activities for agents.
It holds no authority of its own. Each tool calls the Hub as the calling user, and
the Hub decides. Agents see its tools as `console__{tool}` on their Hub connection.
It serves MCP through [veoveo-hub-sdk](https://github.com/VeoveoAI/veoveo-hub-sdk),
which registers it with the Hub and verifies the identity the Hub forwards. It
started from [veoveo-template-mcp-rust](https://github.com/VeoveoAI/veoveo-template-mcp-rust)
`24ae53e`.

## Contents

| Path | Purpose |
|---|---|
| `Dockerfile` | The `dev` environment, the `check` gate, the release `build` and the `runtime` image |
| `openshell/` | The factory environment image and the program that builds and serves a branch |
| `env/profile.d/rust.sh` | Toolchain variables for login shells inside the environment |
| `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml` | The pinned toolchain and code rules |
| `deny.toml` | Dependency licenses, advisories, bans and allowed sources |
| `.config/nextest.toml` | Test profiles for local runs and machine-readable results |
| `.cargo/config.toml` | `SQLX_OFFLINE`, so every build reads the committed query metadata |
| `.sqlx/` | sqlx's checked metadata for the core crates' queries |
| `crates/veoveo-mcp-console/` | The server: its contract, MCP surface, binary, tests and the documents it serves |

## Server

The crate's [`DESIGN.md`](crates/veoveo-mcp-console/DESIGN.md) is its design and
operational contract, and its [`AGENTS.md`](crates/veoveo-mcp-console/AGENTS.md) its
operating manual. The server serves both under `console://docs`.

| Module | Holds |
|---|---|
| `src/contract/` | The `contract` feature: the permission and scope vocabulary, and each tool's request and result |
| `src/mcp/` | One typed `Routes` table for the tools; setup with `server_docs!`; registration derived from the routes |
| `src/server/` | Configuration, registration with the Hub, the database login it issues, the process instance and serving |

The slug `console` is `veoveo_mcp_console::SLUG`; the permission, scope, URI scheme,
documents and key directory derive from it.

At startup the server registers with the Hub, connects with the database login the
Hub issues, holds a process instance, from which the Hub reads its health, and
serves MCP at `{public_base_url}/console/mcp`. It keeps no tables. Process stop
retains the native saved registration for restart; explicit retirement deletes it.

## Tools

Each tool calls the Hub's own route as its caller, at the profile the caller reached
the console through, with a token the SDK exchanges for the caller without
permissions (`HubClient::hub_bearer`, declared by the route builder's `calls_hub`).
The Hub decides; its refusal is the tool's error, with its status and problem detail.

| Tool | Hub route |
|---|---|
| `status(repository, branch, wait?)` | `GET /factory/{profile}/instance?repository=&branch=&wait=`, waiting at most 60 s |
| `logs(repository, branch, since?, limit?)` | `status` without waiting as the access check, then one LogsQL query of the log store for the sandbox ID it names: the workload's `agent` output and the supervisor's network denials, at most 1000 lines |
| `select(server, branch \| null)` | `PUT /selection/{profile}/{server}` with `{"branch": …}`, or `DELETE` for the base branch |
| `watch(repository)` | `PUT /factory/{profile}/watch?repository=` |
| `create_repository(name, id?)` | `POST /lore/{profile}/repositories` with `{"name": …}`; the Hub creates the Lore repository as the caller and assigns the caller its owner, and answers its `id`, `name` and clone `remote`, `<Lore remote>/<id>`. `id` only finishes a creation the Hub answered 503 for with that `id` |
| `computer(restart?)` | `GET /computer/status`, or with `restart: true` `POST /computer/restart`, which the Hub answers before it restarts the caller's Computer at the current version. Its description names the current version and its summary from the Hub's public `GET /computer/version`, read about every 30 s; a change of version changes the description and tells the Hub that the tool list changed |

## Configuration

| Flag | Environment | Default |
|---|---|---|
| `--deployment-id` | `VEOVEO_DEPLOYMENT_ID` | required; a UUIDv7 the Hub supplies with the registration token |
| `--public-base-url` | `VEOVEO_PUBLIC_BASE_URL` | required |
| `--hub-issuer` | `VEOVEO_HUB_ISSUER` | required to serve; the tools call the Hub's routes at its origin |
| `--registration-secret` | `VEOVEO_REGISTRATION_SECRET` | required to serve |
| `--key-directory` | `VEOVEO_KEY_DIRECTORY` | `/var/lib/veoveo/console/keys` |
| `--log-store-url` | `VEOVEO_LOG_STORE_URL` | `http://victorialogs.veoveo-system.svc.cluster.local:9428`, the installation's VictoriaLogs |
| `--port` | `VEOVEO_PORT` | `8805` |
| `--allowed-host` | `VEOVEO_ALLOWED_HOSTS` | none |
| `--allow-loopback-hosts` | `VEOVEO_ALLOW_LOOPBACK_HOSTS` | `false` |

The Hub profile of every call is the one the caller reached the console through, as
the forwarded identity names it. The log store needs no credentials: the Hub's answer
to `status` decides whether the caller may read an instance's output.

## Registration And Offline Description

`VEOVEO_DEPLOYMENT_ID` (`--deployment-id`) is a UUIDv7 shared by registration and
token verification; Hub supplies it with the registration token. Keep it and the
private key directory on ordinary process restart.

The server registers with `VEOVEO_REGISTRATION_SECRET` on its first start and resumes
its saved registration afterwards; process stop does not retire it. When its
declarations change, or the Hub no longer knows the saved registration, it registers
again with the same token and replaces the saved state.

To print the public registration metadata offline, reusing the key in
`--key-directory`, without registering or connecting to Hub or the database:

```sh
veoveo-mcp-console --describe-registration \
  --deployment-id <allocated-uuidv7> \
  --public-base-url https://candidate.example \
  --key-directory /sandbox/builds/<build-id>/keys
```

The SDK serves readiness at `/readyz` once registration and the database connection
succeeded, and reports unavailable if the process instance is lost.

## Build

```sh
docker build --target runtime -t veoveo-mcp-console:<version> \
  --build-arg VERSION=<version> --build-arg REVISION="$(git rev-parse HEAD)" .
```

## Gate

```sh
docker build --target check .
```

The `check` stage checks the `.sqlx` metadata against a PostgreSQL 18.6 database with
the pinned core's kernel migrations, then runs rustfmt, clippy with warnings denied,
cargo-deny, cargo-hack on each feature and the nextest `ci` profile. It runs the same
way on macOS and Linux.

## Factory Instance

`openshell/run` builds the branch and replaces itself with the server, so a branch of
this repository can serve as a factory instance; `.loreignore` keeps `target/` across
`lore sync --reset`. Such an instance reaches the log store only where its sandbox's
policy admits `victorialogs.veoveo-system.svc.cluster.local:9428`; the installation's
base policy does not.

## License

Apache-2.0. See [LICENSE](LICENSE).
