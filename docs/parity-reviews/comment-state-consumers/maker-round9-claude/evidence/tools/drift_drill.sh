#!/bin/bash
# Drift-gate mutation drill on a private export of the tip; every mutation is reverted from pristine copies.
C=~/Library/Caches/rustuml-shared-svg-round9-claude-maker; S=$C/src/drill; O=$C/drift; mkdir -p $O
P=$S/tools/font-chain-reference/colossus-primary-faces; REF=$S/tools/font-chain-reference/monkeyman-round6
GEN=$S/crates/rustuml-render/src/font_chain_coverage.rs; FC=$S/crates/rustuml-render/src/font_chain.rs
rm -rf $O/p.orig $O/ref.orig; cp -a $P $O/p.orig; cp -a $REF $O/ref.orig; cp $GEN $O/gen.orig; cp $FC $O/fc.orig
run() { (cd $S && CARGO_TARGET_DIR=$C/target/drill cargo test -j 6 -p rustuml-render --lib font_chain::tests::coverage_table_is_the_stored_reference_host_extraction > $O/$1.log 2>&1); r=$?; echo "$1 exit $r | $(grep -m1 -E "panicked at|assertion" -A3 $O/$1.log | tr '\n' ' ' | head -c 300)"; }
restore() { rm -rf $P $REF; cp -a $O/p.orig $P; cp -a $O/ref.orig $REF; cp $O/gen.orig $GEN; cp $O/fc.orig $FC; }
# edit <dir> <file> <python expr over text s> ; then re-hash that file in SHA256SUMS when $2 = sums
edit() { python3 -I - "$1" "$2" "$3" "$4" <<'PY'
import sys, hashlib, pathlib
d, name, expr, sums = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4]
p = d / name; s = p.read_text(); t = eval(expr); assert t != s, "mutation did nothing"; p.write_text(t)
if sums == "sums":
    L = (d / "SHA256SUMS").read_text().split("\n"); h = hashlib.sha256(p.read_bytes()).hexdigest()
    (d / "SHA256SUMS").write_text("\n".join((h + "  " + name) if x.endswith("  " + name) else x for x in L))
PY
}
constant() { old=$(shasum -a 256 $O/$2.orig/SHA256SUMS | cut -d' ' -f1); new=$(shasum -a 256 $1/SHA256SUMS | cut -d' ' -f1); sed -i '' "s/$old/$new/" $GEN; }
helv() { echo "s[:s.index('pub(crate) const $1: FaceCoverage')] + s[s.index('pub(crate) const $1: FaceCoverage'):].replace('$2', '$3', 1)"; }
run M0_unmodified
edit $P Helvetica.own.tsv "s.replace('P\t20\t7E', 'P\t20\t7F', 1)" nosums; run M1_own_run_changed; restore
edit $P Helvetica.own.tsv "s.replace('P\t20\t7E', 'P\t20\t7F', 1)" sums; run M2_own_run_and_sums; constant $P p; run M3_own_run_sums_constant; restore
edit $S/crates/rustuml-render/src font_chain_coverage.rs "s[:s.index('const PRIMARY_MAPPED_3')] + s[s.index('const PRIMARY_MAPPED_3'):].replace('(0x20, 0x7E)', '(0x20, 0x7F)', 1)" nosums; run M4_generated_primary_run; restore
edit $S/crates/rustuml-render/src font_chain_coverage.rs "$(helv HELVETICA 'cross_kern: false' 'cross_kern: true')" nosums; run M5_helvetica_kern_bit_table; restore
edit $P Helvetica-Bold.own.tsv "s.replace('cross_kern=0', 'cross_kern=1', 1)" sums; constant $P p; run M6_helvetica_bold_kern_bit_tsv; restore
edit $S/crates/rustuml-render/src font_chain_coverage.rs "$(helv MENLO 'morx: true' 'morx: false')" nosums; run M7_menlo_morx_table; restore
edit $S/crates/rustuml-render/src font_chain_coverage.rs "$(helv VERDANA 'gpos_hebrew_script: false' 'gpos_hebrew_script: true')" nosums; run M8_verdana_gpos_hebr_table; restore
edit $P colossus-font-files.sha256 "s.replace('658718d7', '00000000', 1)" sums; constant $P p; run M9_colossus_helvetica_file_hash; restore
edit $P monkeyman-hb-static.txt "s.replace('c481b239', '00000000', 1)" sums; constant $P p; run M10_reference_lucida_file_hash; restore
edit $S/crates/rustuml-render/src font_chain.rs "s.replace('&[\"Verdana-Bold\"]', '&[]', 1)" nosums; run M11_pending_list_emptied; restore
edit $REF Helvetica.tsv "s.replace('kern=1', 'kern=0', 1)" sums; constant $REF ref; run M12_extraction_helvetica_kern; restore
diff -r $O/p.orig $P && diff -r $O/ref.orig $REF && cmp $O/gen.orig $GEN && cmp $O/fc.orig $FC && echo RESTORED
