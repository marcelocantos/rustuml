# Standing invariants for bullseye_convergence.
#
# Each recipe runs an independent check and exits non-zero on
# violation. Stdout is relayed to the agent verbatim.

.PHONY: bullseye fmt clippy test clean-tree formula-guard

bullseye: fmt clippy test clean-tree formula-guard

fmt:
	@cargo fmt --check && echo "✓ fmt"

clippy:
	@cargo clippy --workspace --quiet -- -D warnings && echo "✓ clippy"

# Capture-and-propagate: a pipeline's status is its last stage, so
# `cargo test | grep` reported grep's (the phrase "test result" appears
# on pass AND fail). Make recipes do not inherit pipefail.
test:
	@out=$$(cargo test --workspace --lib --quiet 2>&1); rc=$$?; \
		printf '%s\n' "$$out" | grep "test result" || true; \
		if [ $$rc -ne 0 ]; then echo "✗ tests"; exit $$rc; fi; \
		echo "✓ tests"

clean-tree:
	@test -z "$$(git status --porcelain)" && echo "✓ clean tree" || \
		(echo "✗ dirty tree"; git status --short; exit 1)

# 🎯T3 guard-rail: if homebrew-tap formula ships rustuml-oracle, it
# must also declare openjdk as a runtime dep (oracle invokes `java`).
formula-guard:
	@FORMULA=$$HOME/work/github.com/marcelocantos/homebrew-tap/Formula/rustuml.rb; \
	if [ ! -f "$$FORMULA" ]; then \
		echo "⚠ formula-guard: $$FORMULA not found, skipping"; \
		exit 0; \
	fi; \
	if grep -q "rustuml-oracle" "$$FORMULA"; then \
		grep -q 'depends_on "openjdk"' "$$FORMULA" || \
			(echo "✗ formula-guard: rustuml-oracle in formula without openjdk dep"; exit 1); \
	fi; \
	echo "✓ formula-guard (🎯T3)"
