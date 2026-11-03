# Standing invariants for bullseye_convergence.
#
# Each recipe runs an independent check and exits non-zero on
# violation. Stdout is relayed to the agent verbatim.

.PHONY: bullseye gate fmt clippy test test-parity check-dirty formula-guard

bullseye: fmt clippy test check-dirty formula-guard

# Run every independent gate even after a failure; make preserves failure status.
gate:
	@$(MAKE) --keep-going fmt clippy test test-parity

fmt:
	@cargo fmt --check && echo "✓ fmt"

clippy:
	@cargo clippy --workspace --quiet -- -D warnings && echo "✓ clippy"

test:
	@cargo test --workspace --lib --quiet --no-fail-fast && echo "✓ tests"

# Full corpus required. No filters or baseline writes are permitted in this gate.
# --no-fail-fast runs the remaining binaries when a parity/evidence suite fails.
test-parity:
	@test -z "$${GOLDEN_FILTER+x}" && test -z "$${HARNESS_WRITE_BASELINES+x}" || \
		{ echo "test-parity requires GOLDEN_FILTER and HARNESS_WRITE_BASELINES unset"; exit 1; }
	@cargo test --workspace --release --no-fail-fast \
		--test golden_no_oracle --test golden_pairs --test harness_guards \
		--test doc_claims --test parity_perturbations --test pipe_mode -- --nocapture

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
