# T14.40 addendum — decoded scalars must not occupy the surrogate-carrier range

Measured before this change, on the independent checker batch CM
(`reference-results.json` SHA256
`07a5679719ea9d84c729183ecaac5281ae2e44bfb48593bf28a1b21f38eb652a`).

`ckg6_amp_last_scalar` is `title Last &#1114111; unit`. Java exits 0.
`SvgGraphics.getTransformer` (`SvgGraphics.java:769-776`) sets
`OutputKeys.ENCODING` to `us-ascii`, and the text node is:

```text
<text fill="#000000" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="78.3057" x="10" y="23.5352">Last &#1114111; unit</text>
```

`StringUtils.manageAmpDiese` (`StringUtils.java:472-483`) matches
`&#([0-9]+);`, calls `Integer.parseInt`, then
`new String(Character.toChars(codePoint))`, and `appendReplacement`
without `quoteReplacement`. `Character.toChars(1114111)` succeeds.
U+10FFFF is one scalar, UTF-16 units U+DBFF U+DFFF. The US-ASCII
transformer writes that pair as the single character reference
`&#1114111;`. `manageUnicodeNotationUplus` (`StringUtils.java:457-469`)
uses the same `Character.toChars` and then `quoteReplacement`.

The same hole is one scalar down. `cr_ent_dec_10fffd.puml` is
`class "Vault &#1114109; end"` (U+10FFFD). The merge base exits 0. The
tip before this addendum exits 1, `An invalid XML character (Unicode:
0xdffd)`.

Rust `java_to_chars` returns `char::from_u32`. U+10F800..=U+10FFFF is
also `java_utf16`'s unpaired-surrogate carrier range
(`java_utf16.rs`, `CARRIER_BASE`). `carried_unit(U+10FFFF)` is 0xDFFF
and `carried_unit(U+10FFFD)` is 0xDFFD, so `svg_export` runs the
`ToStream` surrogate check and rejects a lone low unit. Legitimate
scalars of `ESCAPE`..=U+10FFFF (`ESCAPE` is U+10F7FF) already enter
source text as `ESCAPE` plus the scalar (`java_utf16::escape` and
`java_utf16::text`). `tokens` then reads a character, and the exporter
drops the prefix. `text_width` measures that token as the scalar, not
as a carrier and not as the prefix (`plantuml_metrics.rs`).

This change stores every scalar `Character.toChars` accepts, on both
the decimal and `<U+…>` success paths, with that same prefix when
`java_utf16::reserved` is true. A failed `toChars` (surrogate or above
U+10FFFF) still aborts the whole conversion. `$` and `\` on the decimal
path still abort. Scalars below U+10F7FF stay one raw character. No
witness name is special-cased, and `svg_export` is unchanged.
`compare_svg_strict` already treats `&#N;` and the raw scalar as the
same text, which is how an escaped U+10FFFF can match Java's
`&#1114111;` node, including textLength 78.3057.
