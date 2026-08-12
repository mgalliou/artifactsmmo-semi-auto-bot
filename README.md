## artifactsmmo-semi-auto-bot

Rust implementation of a semi-autonomous bot for the
[ArtifactsMMO](https://artifactsmmo.com/) game.

> [!WARNING]
> This project is a work in progress and may change without notice.

## Architecture

The repository is a Rust workspace split into the generated `openapi` client,
the `api` wrapper, the game-facing `sdk`, bot behavior in `bot`, and the `repl`
and Ratatui-based `tui` frontends. The generated client comes from the official
[OpenAPI specification](https://api.artifactsmmo.com/docs/#/) using
[OpenAPI Generator](https://openapi-generator.tech/).

## Running

The frontend uses `https://api.artifactsmmo.com` and reads the API token from
the `ARTIFACTSMMO_TOKEN` environment variable. The SDK obtains the account name
from the authenticated account-details endpoint; it does not need separate
account-name configuration.

Run the REPL from the repository root so `ArtifactsMMO.toml` and `.cache/`
resolve in the expected location:

```shell
export ARTIFACTSMMO_TOKEN="YOUR_API_TOKEN"
mkdir -p .cache
cargo run -p repl
```

> [!CAUTION]
> Starting the frontend initializes the live client and launches bot threads
> that can issue game actions for every configured character.

## Configuration

`ArtifactsMMO.toml` configures bot and character behavior; it does not contain
the API URL, token, or account name. The file is required because startup
panics if it is missing or invalid.

Character entries are positional and must cover the characters returned by the
account API: the first `[[characters]]` entry configures character 0, the
second configures character 1, and so on.

```toml
# Automatically order upgrades that cannot be sourced immediately.
order_gear = true

# Ignore these catalog items when resolving gear. Already-owned gear is not
# affected; unknown item codes are warned about and ignored.
excluded_items = ["example_item_code"]

[[characters]]
idle = false
is_trader = false
task_type = "monsters" # "monsters" or "items"; defaults to "monsters"
skills = ["combat", "woodcutting"]
goals = [
  "orders",
  { reach_skill_level = { skill = "woodcutting", level = 40 } },
]

[[characters]]
skills = ["mining", "weaponcrafting"]
goals = [
  { follow_max_skill_level = { skill = "weaponcrafting", skill_to_follow = "mining" } },
]
```

All fields inside a character entry are optional. `idle` and `is_trader`
default to `false`, `task_type` defaults to `monsters`, and `skills` and `goals`
default to empty collections. Add one `[[characters]]` block per account
character.

### Skills

Each character can be assigned one or more skills that determine which related
actions it may perform. Available values are `combat`, `mining`, `woodcutting`,
`fishing`, `weaponcrafting`, `gearcrafting`, `jewelrycrafting`, `cooking`, and
`alchemy`.

### Goals

One or more goals can further specify a character's behavior:

- `orders`: try to fulfill orders present on the order board.
- `reach_skill_level`: try to reach a specified skill level.
- `follow_max_skill_level`: level `skill` relative to the highest level reached
  across all characters for `skill_to_follow`.

```toml
[[characters]]
goals = [
  "orders",
  { follow_max_skill_level = { skill = "cooking", skill_to_follow = "fishing" } },
  { reach_skill_level = { skill = "fishing", level = 40 } },
]
```

## Read-Eval-Print-Loop (REPL)

When the bot is running, type `help` to list available REPL commands. Each
command can be entered without arguments to display its usage.

After editing `ArtifactsMMO.toml`, use `config reload` to reload it.
