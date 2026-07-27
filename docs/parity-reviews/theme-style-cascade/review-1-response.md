# Theme/style review-1 response

## Rejected boundary

Review commit `976444af` confirms that `60d16110` preserves directive order
and cross-family legacy aliases, but only integrates a final global style into
four dense renderer structs. That is not the Java model described by the
original account. This response makes the remaining integration obligations
explicit.

## Temporal ownership

Pure CSS entities and links resolve against the style builder captured at
creation. A later CSS declaration cannot repaint an earlier object.

After any legacy `skinparam` command, Java's entity compatibility path consults
the latest diagram builder for every entity. Links remain asymmetric: each link
always retains its creation-time builder. RustUML can reconstruct those
snapshots from the ordered declaration program and each model object's source
line; it must not collapse them into one final family palette.

## Selector ownership

Style resolution is per concrete consumer:

- class body: `root.element.classDiagram.class[.stereotype]`;
- class header: the same signature plus `header`;
- class arrow: `root.element.classDiagram.arrow`;
- equivalent family/element signatures for state, component, and use case.

Stereotypes are normalized by the resolver, not by renderer string equality.
The stereotype priority tier remains higher than a later ordinary declaration.
Header values must be consumed by header text before inherited family/root
values. A dense compatibility field may not reverse that precedence.

## Sparse projection

Adapters project only properties present in the resolved sparse style. Required
consumer properties include body/header font and paint, padding, round corner,
shadowing, document background/margin, and arrow line/font color, thickness,
and line style. Inline entity/link styling remains the final override.

Legacy compatibility values that are not converted into the style program,
such as global `skinparam Padding`, retain their existing dedicated channel.
Their precedence must be based on the Java consumer path, not on which
representation happens to be denser.

## Scheme scope

The current product has no dark-scheme selection input. This integration
therefore claims and tests the regular channel only. The resolver retains dark
fallback semantics for a future product selector, but no adapter may imply that
dark product parity is complete until such a selector exists.

## Acceptance

1. All ten `review1_` perturbations are rerun; every scoped regular-channel
   semantic difference is either exact or reduced to a separately named
   geometry/identity mechanism.
2. Pure-CSS entity/link snapshots and legacy entity-refresh/link-capture
   asymmetry are covered by resolver tests and renderer perturbations.
3. Class stereotypes, header precedence, root document background, arrow font
   paint, and component arrow thickness/style are projected through resolved
   signatures.
4. New parity conversions cite their Java consumer beside the code.
5. The complete no-oracle ratchet has no pass-for-fail swaps.
