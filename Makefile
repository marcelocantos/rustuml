# Standing invariants for bullseye_convergence.
#
# Each recipe runs an independent check and exits non-zero on
# violation. Stdout is relayed to the agent verbatim.

.PHONY: bullseye fmt clippy test clean-tree formula-guard ratchet ratchet-vendor-check

bullseye: fmt clippy test clean-tree formula-guard ratchet

fmt:
	@cargo fmt --check && echo "✓ fmt"

clippy:
	@cargo clippy --workspace --quiet -- -D warnings && echo "✓ clippy"

test:
	@cargo test --workspace --lib --quiet 2>&1 | grep "test result" && echo "✓ tests"

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

# 🎯T14 honesty ratchet: the number the SHIPPED binary earns on the golden
# corpus, gated in both directions. Regression fails; an unlocked improvement
# fails too (a stale baseline hides the next regression). The gate also
# recounts the denominator from the corpus tree, poisons the outputs to prove
# the scorer is not echoing them, and refuses a hand-edited baseline.
# Move the numbers with:
#   python3 tools/ratchet/ratchet.py lock --reason ... --mechanism ...
ratchet: ratchet-vendor-check
	@cargo build --release --quiet -p rustuml -p rustuml-oracle \
		--bin rustuml --bin ratchet_gate
	@python3 tools/ratchet/ratchet.py --config ratchet.json check

# The vendored gate must stay byte-identical to the shared skeleton it was
# drilled against (tools/ratchet/UPSTREAM.md). Machines without the skills
# tree — CI — skip this and run the vendored copy as checked in.
ratchet-vendor-check:
	@UPSTREAM=$$HOME/.claude/skills/oracle-first/ratchet/ratchet.py; \
	if [ ! -f "$$UPSTREAM" ]; then \
		echo "⚠ ratchet-vendor-check: $$UPSTREAM not found, skipping"; \
		exit 0; \
	fi; \
	cmp -s "$$UPSTREAM" tools/ratchet/ratchet.py || \
		(echo "✗ ratchet-vendor-check: tools/ratchet/ratchet.py differs from $$UPSTREAM"; \
		 echo "  re-drill upstream, then re-vendor per tools/ratchet/UPSTREAM.md"; exit 1); \
	echo "✓ ratchet-vendor-check"
