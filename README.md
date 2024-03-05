# fetch-sfs
Code moved to <https://github.com/spraakbanken/swegov-opendata-rs>.

Program for collecting Svensk Författningssamling from https://data.riksdagen.se

## Build

You'll need to have a some what recent version of Rust installed.

Then run `cargo build --release`.

The built binary is in `target/release/fetch-sfs`.

## Run

`fetch-sfs` can be run as is when built.

### Configuration

- **Logging** By default, `fecth-sfs` logs all messages for level `INFO` and above, logging is made to `stderr` in json lines (new-lined separated json). Set logging level with `RUST_LOG=<desired level> ./fetch-sfs`
    - `RUST_LOG=trace ./fetch-sfs` is very verbose, writes all log messages for all libraries.
    - `RUST_LOG=fetch_sfs=trace,warn ./fetch-sfs` is quite verbose, writes all log messages for `fetch_sfs` but only messages on level `WARN` for other libraries.
- **Output path** By default, `fetch-sfs` writes all collected pages as json files in `./output`, this can be changed by setting the key `sfs.config_path` in a [configuration file](#configuration-file).
- **User agent** By default, the user agent is set to `fetch-sfs/0.1.0`, this can be changed by set the key `sfs.user_agent` in a [configuration file](#configuration-file).


#### Configuration file
Configurations can be given in a file called  `config.json` that is placed in the current directory.

Example (with the default values):
```json
{
    "sfs": {
        "user_agent": "fetch-sfs/0.1.0",
        "output_path": "./output"
    }
}
```
