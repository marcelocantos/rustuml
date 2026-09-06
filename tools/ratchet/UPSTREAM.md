# Vendored: honesty-ratchet gate

`ratchet.py` is a verbatim copy of the shared oracle-first skeleton.

| | |
|---|---|
| Upstream | `~/.claude/skills/oracle-first/ratchet/ratchet.py` (mirrored at `marcelocantos/skills`) |
| sha256 | `690a28c9a4df3f0678432e36571aa2288f1668dee33b10680b680532bbd07524` |
| Vendored | 2026-09-06 |
| Doctrine | `~/.claude/skills/oracle-first/honesty-ratchet.md` |
| Drill | `~/think/burst-2026-09/ratchet-drill/drill.py` — 10/10 against this sha256 |

It is vendored rather than referenced because CI has no `~/.claude`, and a
gate that only runs on one machine is not a standing enforcement point.

Do not edit this copy. Fix upstream, re-run the drill, then re-vendor and
update the hash above; `make ratchet-vendor-check` fails when the two
diverge on a machine that has the upstream tree.
