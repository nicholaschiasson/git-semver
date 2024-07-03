# git-semver

[![github](https://img.shields.io/badge/github-nicholaschiasson/git--semver-default?logo=github)](https://github.com/nicholaschiasson/git-semver)
[![crates.io](https://img.shields.io/crates/v/git-semversion?logo=rust)](https://crates.io/crates/git-semversion)
[![docs.rs](https://img.shields.io/docsrs/git-semversion?logo=docs.rs)](https://docs.rs/git-semversion)
[![build](https://github.com/nicholaschiasson/git-semver/actions/workflows/build.yml/badge.svg)](https://github.com/nicholaschiasson/git-semver/actions/workflows/build.yml)
[![license](https://img.shields.io/github/license/nicholaschiasson/git-semver?logo=opensourceinitiative&logoColor=white)](https://github.com/nicholaschiasson/git-semver?tab=MIT-1-ov-file#readme)

Generate a semantic versioning compliant tag for your HEAD commit.

## CLI

This project also publishes a binary application for use on the command line.

### Installation

For now, crates.io is the only place this is being distributed.

```
cargo install git-semversion
```

### Usage

```
Generate a semantic versioning compliant tag for your HEAD commit

Usage: git-semver [OPTIONS]

Options:
  -m, --main-branch <MAIN_BRANCH>
          The name of your repository's main branch. Useful if you continue to use "master" or "trunk" [default: main]
  -p, --prerelease-id <PRERELEASE_ID>
          Identifier to use for prerelease during non-main branch execution, using branch name slug when omitted
  -r, --prerelease-revision <PRERELEASE_REVISION>
          Revision to use for prerelease during non-main branch execution, using short commit hash when omitted
  -i, --increment <INCREMENT>
          Explicit increment level override for use during main branch execution, forcing to ignore the increment level derived from commit summary [possible values: patch, minor, major]
      --default-increment <DEFAULT_INCREMENT>
          Increment level override for non-merge commits to main branch, ie. commits directly to main branch [default: patch] [possible values: patch, minor, major]
  -e, --match-expression <MATCH_EXPRESSION>
          Regular expression to match the increment level in the commit summary of a commit to the main branch [default: "^Merge .*(patch|minor|major)/[\\w-]+"]
  -h, --help
          Print help
  -V, --version
          Print version
```

## Docker

This project also publishes a docker image, exposing the CLI tool.

### Installation

You can pull the image from GitHub's container registry:

```
docker pull ghcr.io/nicholaschiasson/git-semver:latest
```

Or for more convenience, you can reference the image in a docker compose file:

```yaml
---
services:
  git-semver:
    image: ghcr.io/nicholaschiasson/git-semver:latest
    volumes:
      - .:/repo:ro
```

For extra convenience, you can create an alias to the docker compose command:

```
echo 'alias git-semver="docker compose --file path/to/docker-compose.yml run --rm git-semver"' >> "${HOME}/.bashrc"
source "${HOME}"/.bashrc
```

After that, you should be able to simply run `git-semver` to invoke the container.

### Usage

The docker image entrypoint is the git-semver CLI binary itself, meaning the usage is the exact same as indicated above.

## Development

### Prerequisites

- [nix](https://nixos.org/download.html)
- [nix flakes](https://nixos.wiki/wiki/Flakes#Enable_flakes)

### How-to

Create the development shell environment. Necessary to run all other commands.

```shell
nix develop
```

Build with cargo.

```shell
just build
```

Check the code with cargo's built-in fast static analysis.

```shell
just check
```

Remove build files.

```shell
just clean
```

Format the code.

```shell
just format
```

Check the code with clippy for better static analysis.

```shell
just lint
```

Run the application.

```shell
just run
```

Run tests with cargo's built-in test runner.

```shell
just test
```

Watch for code changes and rebuild.

```shell
just watch
```

All `just` commands can accept additional command line arguments after a `--`.

For example: run the application with a flag to report the version.

```shell
just run -- --version
```

#### Tips and Recommendations

##### Open IDE from Development Shell

To get linking to rust binaries in your IDE, you should open the development shell from your terminal and then open your IDE
from that shell session. This will carry over the development shell's environment into your IDE.

For example if you work with VSCode.

```shell
cd path/to/this/project
nix develop
code .
```

By doing this, you can install the rust-analyzer VSCode extension and it will work properly since it will be able to point to
the correct rust binaries and libraries. You will also have access in VSCode to any packages installed by the nix flake.

## To Do

- [x] Dockerfile
- [ ] Tests
- [ ] Github Action
