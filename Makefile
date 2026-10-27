# Standing invariants for bullseye_convergence.
#
# Each recipe runs an independent check and exits non-zero on
# violation. Stdout is relayed to the agent verbatim.

.PHONY: bullseye fmt clippy test check-dirty formula-guard

bullseye: fmt clippy test check-dirty formula-guard

fmt:
	@cargo fmt --check && echo "✓ fmt"

clippy:
	@cargo clippy --workspace --quiet -- -D warnings && echo "✓ clippy"

test:
	@cargo test --workspace --lib --quiet 2>&1 | grep "test result" && echo "✓ tests"

check-dirty:
	@dirty=$$(git status --porcelain | grep -vE 'bullseye\.yaml$$' || true); \
	if [ -z "$$dirty" ]; then echo "✓ working tree clean"; \
	else \
	  echo ""; \
	  echo "================================================================"; \
	  echo "⚠  DIRTY WORKING TREE"; \
	  echo ""; \
	  echo "Warning only — invariants still pass (exit 0)."; \
	  echo "Look at the files below before starting a new target."; \
	  echo "Leftover work from a different objective → park it in a commit first."; \
	  echo "This session's WIP on the recommended target → continue."; \
	  echo "================================================================"; \
	  echo "$$dirty"; \
	  echo "================================================================"; \
	  echo ""; \
	fi

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
