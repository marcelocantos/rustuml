// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Layout budget, failure reasons, and the record of diagrams that fell
//! back because layout did not finish.
//!
//! A diagram whose layout exceeds its budget still renders: every caller
//! has a grid fallback. That fallback used to be silent, so a large
//! diagram came out arranged in a grid with nothing anywhere saying the
//! real layout had been abandoned — the user saw a wrong diagram rather
//! than a slow one. Everything here exists to make that visible: the
//! failure carries its reason, the caller records it, a warning reaches
//! stderr, and the rendered SVG is annotated.
//!
//! The record is thread-local because rendering is: `layout_full`
//! returns to the calling thread and the caller records there, so
//! concurrent renders of different diagrams never mix notices.

use std::cell::RefCell;
use std::fmt;
use std::sync::LazyLock;
use std::time::Duration;

/// Wall-clock budget for one Graphviz layout when nothing overrides it.
///
/// Graphviz `dot` is superlinear in node count on this corpus: a
/// synthetic tree measures 0.53 ms at 10 nodes, 29 ms at 50 and 9.13 s
/// at 200 (`cargo bench -p rustuml-bench`, `layout/`). Five seconds
/// therefore sits inside the region where the curve is turning, which is
/// why it is a budget with a reported fallback rather than a limit
/// chosen to be generous. Raise it for one run with
/// [`BUDGET_ENV`] rather than editing this.
pub const DEFAULT_BUDGET: Duration = Duration::from_secs(5);

/// Environment variable overriding [`DEFAULT_BUDGET`], in milliseconds.
pub const BUDGET_ENV: &str = "RUSTUML_LAYOUT_BUDGET_MS";

/// Why a layout produced no positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutFailure {
    /// Graphviz was still working when the budget ran out. The layout
    /// keeps running on the worker thread; its result is discarded.
    TimedOut { nodes: usize, timeout: Duration },
    /// Graphviz panicked. The worker survives; this caller gets nothing.
    Panicked { nodes: usize },
}

impl fmt::Display for LayoutFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutFailure::TimedOut { nodes, timeout } => write!(
                f,
                "layout of {nodes} nodes exceeded the {} ms budget",
                timeout.as_millis()
            ),
            LayoutFailure::Panicked { nodes } => {
                write!(f, "layout of {nodes} nodes failed inside Graphviz")
            }
        }
    }
}

/// The budget honoured by production call sites: the test override if one
/// is set on this thread, else [`BUDGET_ENV`], else [`DEFAULT_BUDGET`].
pub fn budget() -> Duration {
    if let Some(d) = OVERRIDE.with(|o| *o.borrow()) {
        return d;
    }
    static FROM_ENV: LazyLock<Option<Duration>> = LazyLock::new(|| {
        std::env::var(BUDGET_ENV)
            .ok()?
            .parse::<u64>()
            .ok()
            .map(Duration::from_millis)
    });
    FROM_ENV.unwrap_or(DEFAULT_BUDGET)
}

thread_local! {
    static OVERRIDE: RefCell<Option<Duration>> = const { RefCell::new(None) };
    static NOTICES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Sets the budget for this thread, for tests that need a failure without
/// waiting for one. Pass `None` to fall back to the environment.
pub fn set_budget_override(budget: Option<Duration>) {
    OVERRIDE.with(|o| *o.borrow_mut() = budget);
}

/// Records that `what` (a diagram kind, e.g. `"class diagram"`) fell back
/// because layout did not finish, and warns on stderr. Call this at every
/// site that turns a [`LayoutFailure`] into a grid or stacked fallback.
pub fn record(what: &str, failure: LayoutFailure) {
    let notice = format!("{what}: {failure}; rendered with the grid fallback");
    eprintln!("rustuml: {notice}");
    NOTICES.with(|n| n.borrow_mut().push(notice));
}

/// Number of notices recorded on this thread so far. Pair with
/// [`notices_since`] around a render to find what that render fell back on.
pub fn mark() -> usize {
    NOTICES.with(|n| n.borrow().len())
}

/// Notices recorded on this thread since `mark`.
pub fn notices_since(mark: usize) -> Vec<String> {
    NOTICES.with(|n| n.borrow().get(mark..).unwrap_or_default().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_override_wins_and_clears() {
        set_budget_override(Some(Duration::from_millis(7)));
        assert_eq!(budget(), Duration::from_millis(7));
        set_budget_override(None);
        assert_eq!(budget(), DEFAULT_BUDGET);
    }

    #[test]
    fn notices_accumulate_from_the_mark() {
        let start = mark();
        record(
            "class diagram",
            LayoutFailure::TimedOut {
                nodes: 200,
                timeout: Duration::from_secs(5),
            },
        );
        let notices = notices_since(start);
        assert_eq!(notices.len(), 1);
        assert!(notices[0].contains("200 nodes"), "{}", notices[0]);
        assert!(notices[0].contains("5000 ms"), "{}", notices[0]);
        assert!(notices[0].contains("grid fallback"), "{}", notices[0]);
    }
}
