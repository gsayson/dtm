# Djinn Toolchain Manager

This tool manages installations of the Djinn toolchain/CLI.

## Usage

- `dtm install <VERSION>` installs the Djinn CLI release of the specific version. The version is essentially
just the tag, without the prefixing `v`. To install the release tagged `v1.0.0`, enter `dtm install 1.0.0`.
- `dtm install` without the arguments looks up the latest release on GitHub.
- `dtm use <VERSION>` rewrites the shell scripts at the *DTM home* (see `dtm list`), to call the desired toolchain.
Unfortunately, it does not set itself up for the `PATH` variable; please add an entry of the *DTM home* directory into the
`PATH` variable.
- `dtm list` lists the *DTM home*, where DTM stores the shell scripts and the toolchains, along with all
installed toolchains.