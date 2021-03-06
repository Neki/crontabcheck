# crontabcheck

A simple crontab validator. Can validate crontab files before they are actually deployed.

Should work with most standard crons. When in doubt, this validator should err on the side of false positives (rejecting valid crontabs).

Note that the only foolproof way to actually validate a crontab is to install it and check whether cron can
read it, typically by looking at `/var/log/syslog`.

[![Build Status](https://travis-ci.org/Neki/crontabcheck.svg?branch=master)](https://travis-ci.org/Neki/crontabcheck)

## Usage

```
crontabcheck < /etc/cron.d/yourcrontab
```

Will exit with status code 0 (and no output) if the crontab file is valid. Otherwise, will exit with a
non-zero status code and print to stdout the (hopefully not too cryptic) error messages.

Use `crontabcheck --help` for the list of options. You'll probably want to use the `-p` flag (read valid usernames from `/etc/password`) or the `-u` option (to provide a comma-delimited list of valid usernames).

This tool will catch errors such as invalid dates, typo in usernames, command lines exceeding the maximum supported length...

## Installation

The [Github releases page](https://github.com/Neki/crontabcheck/releases) lists available binaries built on
Ubuntu 14.04 (from Travis CI). They should work on a variety of Debian-like distributions.

Alternatively, you can compile from source. Instructions are below.

### Limitations

* Only supports the `/etc/cron.d` format. No user crontabs.
* No Unicode support (or support for anything outside of ASCII), but you probably shouldn't embed non-ASCII characters in your crontabs anyway.
* Instead of trying to deal with `%`, will simply error out if it encounter this character. Save yourself from suprises and don't use this cron feature :)

If someone needs one of the above, this should not be too hard to add. Just open a Github issue, or a pull request.

## Build from source

You need to have a Rust toolchain installed on your system. See instructions at https://www.rustup.rs/.

Build the project with:

```
cargo build --release
```

The binary will be in `target/release/`.

## Development

If not already done, you need to install a Rust toolchain (as described in the "Build from source" section).

```
# build the project in debug mode
cargo build

# run tests
cargo test
```

Contributions are welcome ; use Github pull requests.

## Questions? Issues?

Use Github issues.

## Motivation & disclaimer

This was primarily a pretext to learn Rust, and play with a parser library while I was at it.

The approach used (parser combinator lib) is probably not the optimal one:
* error messages could be clearer
* the code could be simpler
* Rust is probably not the best language for such a simple tool

The end result is useful however ; my practical use case was to validate crontab files before deploying them with Ansible.

## License

Licensed under the MIT license. See the LICENSE file for details.
