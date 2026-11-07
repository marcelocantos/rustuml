# RustUML patch of svg2pdf 0.13.0

This is the crates.io package `svg2pdf` 0.13.0 (MIT OR Apache-2.0),
copied so RustUML can keep PDF bytes stable. Upstream writes resource
dictionaries and font objects by iterating a `HashMap`. That order
changes from process to process. RustUML sorts resource entries by
allocated name and fonts by reference id before writing them. No other
behavior is changed.

The workspace `[patch.crates-io]` entry points `svg2pdf` here.
