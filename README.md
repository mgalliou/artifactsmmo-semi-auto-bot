## artifactsmmo-semi-auto-bot

Rust implementation of a semi-autonomous bot for the
[ArtifactsMMO](https://artifactsmmo.com/) game.

> :warning: Keep in mind that this is a work in progress and that things will
> probably not work the way you expect it to, based on what is described in this
> `README`. :warning:

## Architecture

The bot is built around a
[client](https://github.com/mgalliou/artifactsmmo-openapi) generated from the
official [OpenAPI specification](https://api.artifactsmmo.com/docs/#/) with the
following [openapi-generator](https://openapi-generator.tech/).

## Configuration

The bot must be configured with the `ArtifactsMMO.toml` file.

You have to configure at least the `base_url` and the `token`:

```toml
base_url = "https://api.artifactsmmo.com"
token = "YOUR_API_TOKEN"
```

The behavior of each character can be configured using the `characters` array
(see following sections).

### Skills

Each character can be assigned one or multiple skills to determine if it is
allowed to do actions related to those skills.

```toml
# char 1
[[characters]]
skills = ["combat", "woodcutting"]
# char 2
[[characters]]
skills = ["mining", "weaponcrafting"]
# char 3
[[characters]]
skills = ["mining", "gearcrafting"]
# char 4
[[characters]]
skills = ["mining", "jewelrycrafting"]
# char 5
[[characters]]
skills = ["cooking", "fishing", "alchemy"]
```

### Goals

Goal can be assigned to characters to further specify the behavior of the
character. For now there is four kind of goals:

- `orders`: the character will try to fulfill to `orders` present on the
  `orderboard`.
- `reach_skill_level`: the character must reach a certain level of a skill
- `follow_max_skill_level`: the character will try to level the given `skill` so
  that it follows follow the highest level reached across all characters for the
  `skill_to_follow`.

One or multiple goals can be assigned to each characters.

```toml
[[characters]]
# ...
goals = [
  "orders",
  { "follow_max_skill_level" = { skill = "cooking", skill_to_follow = "fishing" } },
  { "reach_skill_level" = { skill = "fishing", level = 40 } },
```

## Read-Eval-Print-Loop (REPL)

When the bot is running, you can interact with it using the `REPL` by typing
commands. The `help` command will list all available commands. Each command can
be entered without arguments to display its usage.
