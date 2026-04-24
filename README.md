# lls-addon-install

`lls-addon-install` is a Rust CLI for installing LuaLS addons from the official `LuaLS/LLS-Addons` registry into a local addon directory, then updating `.luarc.json` so LuaLS can discover them.

## Warning

This project was generated with AI assistance.

You should review the source code, test the behavior in your own environment, and verify that the generated `.luarc.json` changes match your project's expectations before relying on it in production or team workflows.

## What It Does

The tool follows the LuaLS addon workflow instead of copying the `LLS-Addons` index repository itself:

1. Fetch the official `LLS-Addons` registry metadata from `.gitmodules`
2. Resolve addon names such as `openresty` or `love2d` to their real Git repositories
3. Clone each addon into `--target/<addon-name>`
4. Remove the cloned `.git` directory so the result is a plain addon folder
5. Update `.luarc.json` and append the addon root directory to `workspace.userThirdParty`

This matches the LuaLS model where multiple addons live under one parent directory and that parent directory is referenced by `workspace.userThirdParty`.

## Features

- Install one or more addons by registry name
- List available addons from the official registry
- Replace existing addon directories with `--force`
- Update `.luarc.json` automatically by default
- Support JSONC in `.luarc.json`, including comments and trailing commas
- Preserve existing JSONC comments and formatting as much as possible via `jsonc-parser` CST editing

## Requirements

- Rust
- `git`

## Build

```bash
cargo build --release
```

## Usage

Install one or more addons:

```bash
lls-addon-install --target ./LuaAddons openresty love2d
```

Install an addon and update a specific `.luarc.json`:

```bash
lls-addon-install --target ./LuaAddons --luarc /path/to/project/.luarc.json openresty
```

List addons from the registry:

```bash
lls-addon-install --target ./LuaAddons --list
```

Replace an existing installation:

```bash
lls-addon-install --target ./LuaAddons --force openresty
```

Skip `.luarc.json` updates:

```bash
lls-addon-install --target ./LuaAddons --no-luarc openresty
```

Show help:

```bash
lls-addon-install --help
```

## `.luarc.json` Behavior

If `.luarc.json` does not exist, the tool creates one like this:

```json
{
  "workspace.userThirdParty": [
    "/path/to/project/LuaAddons"
  ]
}
```

Behavior details:

- If `workspace.userThirdParty` already exists, the target path is appended instead of overwritten.
- If the file uses the legacy key `Lua.workspace.userThirdParty`, that key is preserved and updated.
- `.luarc.json` is parsed as JSONC, so comments and trailing commas are accepted.
- The file is edited with `jsonc-parser` CST APIs, which helps preserve existing comments and formatting where possible.

## References

- LuaLS addon documentation: https://luals.github.io/wiki/addons/
- LuaLS configuration documentation: https://luals.github.io/wiki/configuration/
- Official addon registry: https://github.com/LuaLS/LLS-Addons
