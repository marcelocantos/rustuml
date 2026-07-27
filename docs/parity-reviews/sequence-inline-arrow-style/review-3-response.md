# Sequence review-3 mechanism response

## Scope

Review commit `67a51897` rejected `747e138f` with three independent semantic
counterexamples plus a broader physical-font divergence. This response narrows
the next production change to the three named mechanisms. Physical-font metric
parity remains a separate renderer-wide concern and is not claimed here.

## Half-arrow direction

At PlantUML revision `71806a2`, `CommandArrow` treats a left slash or backslash
dressing as `reverseDefine` even when the token contains no `<`. It derives
decoration ownership from the written token, then swaps semantic participants.

RustUML currently derives reverse direction only from `<`. A token beginning
with `/`, `//`, `\`, or `\\` must therefore be classified as right-to-left
before source/target decorations and participant ownership are calculated.
This is a grammar property of the arrow token, not a label or topology rule.

## Autonumber decimal formats

`CommandAutonumber` passes the complete format to `DecimalFormat`, and
`DottedNumber.format` retains literal text surrounding the numeric placeholder.
Angle brackets do not make a numeric format into Creole markup. Both
`<00000>` and `<ID-00000>` are literal decimal-format templates.

RustUML must distinguish recognized Creole tags from an otherwise literal
angle-delimited token containing a decimal placeholder. The same parsed
template must drive emitted text, styled runs, and width measurement.

## External live segment at an ordinate

`MessageExoArrow#getLeftStartInternal` and `getRightEndInternal` call
`LivingParticipantBox#getLiveThicknessAt` at the arrow's exact y. `LifeLine`
defines the resulting segment as:

- inactive: `[center, center]`;
- active depth `d`: `[center - half_width, center + d * half_width]`.

All activation variations at the same ordinate have already changed the
staircase level queried there. This includes a message's inline `++`/`--` and
immediately following standalone `activate`/`deactivate` events. Consequently:

- left-boundary messages use the participant segment's left endpoint;
- right-boundary messages use its right endpoint;
- repeated labels or the number of earlier external messages are irrelevant.

RustUML must compute the live depth at the message ordinate once from event
semantics and use those segment endpoints for all four external directions.
It must not infer the endpoint from a one-step deactivation flag or a fixed
single activation shift.

## Acceptance

1. All 20 preserved perturbations remain strictly equivalent.
2. The two review-4 half-arrow spatial directions are strictly equivalent.
3. Prefixed and unprefixed angle autonumber templates retain literal text and
   decimal padding through stop/resume.
4. Both shared-border live-depth fixtures are strictly equivalent.
5. New unit tests perturb labels, counter widths, nesting depth, and same-y
   activation order.
