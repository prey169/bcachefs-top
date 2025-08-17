# bcachefs-top
This a passion project to replace the built in `bcachefs fs top` command 
with something that shows more than just fs stats all in one view.

Currently, only works with the first matching bcachefs filesystem.

I hope to have time to add to this to make it into its final form
which will one day show plenty of disk statistics, plus several
interesting usage stats from `bcachefs fs usage`.

# features
Currently this has 2 features.
  1. A scrollable, sortable by alphabetical or differential values version
of `bcachefs fs top`.
  2. The ability to pipe out json stats of the current bcachefs counters
as is, or within a duration of X seconds. 

# setup
either: `cargo install bcachefs-top`
or download this repo and `cargo build --release`


# usage
```
Usage: bcachefs-top [OPTIONS]  [-- [PATH]]

Arguments:
  [PATH] 

Options:
  -j, --json
  -t, --time <TIME>  [default: 2]
  -h, --help         Print help
```

When running in top mode (non json mode):
```
 - j, down arror, or scroll wheel down to scroll down
 - k, up arrow, or scroll wheel up to scroll up
 - s to toggle through sorted by alphabetical ascending or diffs by decending
 - q or ctrl-c to quit
```

# Planned features
 - Adding disk statistics, such as io latency
 - Adding some stats from `bcachefs fs usage`
 - Support for multiple bcachefs filesystem stats
 - Test cases so I can properly make sure things work as expected
 - A helpful command bar so it is obvious what keys do what
 - Better granularity when it comes to some stats, similar to the probes that `bcachefs fs top` currently has

Consider this in super alpha and please provide suggestions
or bug reports as issues.

Thank you!
