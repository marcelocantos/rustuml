# Class link labels — CT addendum

Batch CT scored the held-out probes. One valid case is still a diff because
of this packet. The decoration-token rejections are older.

## Valid failure

`g4f_url_pipe` is `pre\n[[http://example.com/a|la\nbel]]\npost`. Java exits 0
and draws two link lines, `http://example.com/a|la` and `bel`, both pointing
at `http://example.com/a|la\nbel`. The tip draws `la` and `bel` pointing at
`http://example.com/a` (DIFF 8). The merge base is DIFF 15.

`UrlBuilder` does not split a link on `|`. The label separator is
whitespace. With no whitespace, the whole inner string is the URL, `getLabel`
falls back to that URL, and `Display.getWithNewlines` splits it.
`relationship_link_parts` splits on `|` first, so the URL never reaches the
newline splitter.

## Not this packet

Every ortho pair whose left token is `|{`, `o|`, or `o{` is a Java syntax
error at line 5. The merge binary also exits 0 on all 24. `CommandLinkClass`
takes those tokens from `LinkDecor.getRegexDecors1`, which lists `}`, `}o`,
`|o`, `||`, and `}|` on the left. The right-side tokens `|{`, `o|`, and `o{`
belong to `getRegexDecors2`. Rust's `([|o}][|{o])--([|o][|{])` admits them on
the left. That regex was not part of this edit.

`g4f_ortho_tb_dl_ob` (`||--o{`, top to bottom) is DIFF 10 on the tip and on
the merge base. The remaining gap is the vertical route, which this packet
does not refit.
