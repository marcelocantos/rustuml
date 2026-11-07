# T14.40 addendum — do not re-encode every non-ASCII text node

Measured after the draft, before keeping it. `cargo test -p rustuml-render --lib` with decimal `&#N;` encoding inside `write_text_element` failed 7 tests:

- `class::tests::empty_package_nodes_emit_visible_metadata_driven_chrome` (`class.rs:15885`)
- `object::tests::linked_stereotypes_use_the_painted_svek_y_envelope` (`object.rs:3221`)
- `sequence::tests::every_participant_paints_filtered_ordered_display` (`sequence.rs:17000`)
- `sequence::tests::queue_stereotype_display_survives_bare_and_created_consumers` (`sequence.rs:16380`)
- `usecase::tests::actor_stereotype_is_a_measured_top_tile_for_renamed_actor` (`usecase.rs:4758`)
- `usecase::tests::renamed_actorless_labels_expand_the_complete_svek_painted_envelope` (`usecase.rs:4657`)
- `usecase::tests::renamed_relation_labels_use_svek_solved_table_origins` (`usecase.rs:4628`)

They search the SVG for the raw guillemets U+00AB / U+00BB. `sequence.rs` and `class.rs` are outside this lane, so those assertions cannot be rewritten here.

The AtomText conversion still stores the scalar. `compare_svg_strict` decodes `&#N;` and the raw scalar to the same text node, which is the golden and perturbation oracle. Whole-CLI bytes of a non-ASCII scalar remain a transformer-encoding residual (Java's US-ASCII writer versus this raw text node). This draft does not add a second encoder to chase that byte difference.

`svg_export` still rewrites a raw CR to `&#13;`. ASCII references are unchanged by this decision.
