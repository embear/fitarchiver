# FIT file archiver

Rename and move/copy FIT files based on activity data to a destination that can
be given as a template string.

## Installation

Clone the repository and install using cargo:

```sh
cargo install --path <path to repository>
```

Optionally run the unit tests:

```sh
cargo test
```

## Usage

The most current description of fitarchiver can be retrieved with `fitarchiver --help`:

```
Rename FIT files based on activity data and copy it to a directory based on year and month of the activity.

Usage: fitarchiver [OPTIONS] <files>...

Arguments:
  <files>...
          List of FIT files to archive.

Options:
  -d, --directory <archive directory>
          Base directory where the archive is created.
          
          [default: .]

  -f, --file-template <template string>
          Format string defining the path and name of the archive file inside the
          archive directory. '/' must be used as a separator for path components.
          For expanding the timestamp of the workout all tags of strftime() are
          supported. In addition to those the tags the following FIT file specific
          conversions are supported:
          
            Tag   Description     Example          Default
            ------------------------------------------------
            $s    sport type      'running'        'unknown'
            $S    sport subtype   'trail'          'unknown'
            $n    sport name      'trail_run'      'unknown'
            $w    workout name    'temporun_8km'   'unknown'
          
          [default: %Y/%m/%Y-%m-%d-%H%M%S-$s]

  -m, --move
          Move files to archive instead of copying them.

  -n, --dry-run
          Do not copy or move the files, just show what will happen.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Example:

```sh
fitarchiver -d ~/backup/activities ~/Downloads/*.fit
```
