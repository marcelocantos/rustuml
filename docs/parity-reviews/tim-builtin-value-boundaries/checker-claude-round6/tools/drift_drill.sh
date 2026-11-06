#!/bin/bash
# Drift-gate mutation drill on the private tip export; every mutation is reverted from a pristine copy.
C=~/Library/Caches/rustuml-shared-svg-round6-claude-checker; S=$C/src/7e9b16f52; O=$C/drift; mkdir -p $O
REF=$S/tools/font-chain-reference/monkeyman-round6; GEN=$S/crates/rustuml-render/src/font_chain_coverage.rs
cp -a $REF $O/ref.orig; cp $GEN $O/gen.orig
run() { (cd $S && CARGO_TARGET_DIR=$C/target/7e9b16f52 cargo test --release -j 6 -p rustuml-render --lib font_chain::tests::coverage_table_is_the_stored_reference_host_extraction > $O/$1.log 2>&1); r=$?; echo "$1 exit $r $(grep -m1 -oE 'panicked at [^ ]*|assertion[^\n]{0,120}' $O/$1.log | head -1) | $(grep -m1 -E 'left|right|simple|complex|SHA256' $O/$1.log | head -c 160)"; }
restore() { rm -rf $REF; cp -a $O/ref.orig $REF; cp $O/gen.orig $GEN; }
run M0_unmodified
# M1: one stored run changed, SHA256SUMS untouched
sed -i '' '0,/^S\t/s/^S\t\([0-9A-F]*\)\t\([0-9A-F]*\)$/S\t\1\t\2X/' $REF/LucidaGrande.tsv 2>/dev/null; python3 - "$REF/LucidaGrande.tsv" <<'P'
import sys; p=sys.argv[1]; L=open(p).read().split("\n")
i=[k for k,l in enumerate(L) if l.startswith("S\t")][3]; f=L[i].split("\t"); f[2]=format(int(f[2],16)+1,"X"); L[i]="\t".join(f); open(p,"w").write("\n".join(L))
P
run M1_tsv_changed; restore
# M2: run changed and its SHA256SUMS line updated (constant not)
python3 - "$REF" <<'P'
import sys, hashlib, pathlib; R=pathlib.Path(sys.argv[1]); p=R/"LucidaGrande.tsv"; L=p.read_text().split("\n")
i=[k for k,l in enumerate(L) if l.startswith("S\t")][3]; f=L[i].split("\t"); f[2]=format(int(f[2],16)+1,"X"); L[i]="\t".join(f); p.write_text("\n".join(L))
s=(R/"SHA256SUMS").read_text().split("\n"); h=hashlib.sha256(p.read_bytes()).hexdigest()
s=[(h+"  LucidaGrande.tsv") if x.endswith("  LucidaGrande.tsv") else x for x in s]; (R/"SHA256SUMS").write_text("\n".join(s))
P
run M2_tsv_and_sums; 
# M3: also update the recorded SHA256SUMS hash in the generated table
NEW=$(shasum -a 256 $REF/SHA256SUMS | cut -d' ' -f1); sed -i '' "s/1c345cc384f0edb939dac3b87cf135db67ed7b6f8ffb76b586a8f9fd8480a6ed/$NEW/" $GEN
run M3_tsv_sums_constant; restore
# M4: generated table run changed only
python3 - "$GEN" <<'P'
import sys,re; p=sys.argv[1]; s=open(p).read(); s=s.replace("(0x38B, 0x38B)", "(0x38B, 0x38C)", 1); open(p,"w").write(s)
P
run M4_generated_run; restore
# M5: Helvetica fallback_positions_marks flipped in the generated table
python3 - "$GEN" <<'P'
import sys; p=sys.argv[1]; s=open(p).read(); a=s.index("pub(crate) const HELVETICA: FaceCoverage"); b=s.index("};", a)
s=s[:a]+s[a:b].replace("fallback_positions_marks: true","fallback_positions_marks: false")+s[b:]; open(p,"w").write(s)
P
run M5_helvetica_fallback_flip; restore
# M6: Menlo zeroes_unmapped_marks flipped
python3 - "$GEN" <<'P'
import sys; p=sys.argv[1]; s=open(p).read(); a=s.index("pub(crate) const MENLO: FaceCoverage"); b=s.index("};", a)
s=s[:a]+s[a:b].replace("zeroes_unmapped_marks: true","zeroes_unmapped_marks: false")+s[b:]; open(p,"w").write(s)
P
run M6_menlo_zero_flip; restore
# M7: Lucida Grande fallback_positions_marks flipped (kern=0 face)
python3 - "$GEN" <<'P'
import sys; p=sys.argv[1]; s=open(p).read(); a=s.index("pub(crate) const LUCIDA_GRANDE: FaceCoverage"); b=s.index("};", a)
s=s[:a]+s[a:b].replace("fallback_positions_marks: true","fallback_positions_marks: false")+s[b:]; open(p,"w").write(s)
P
run M7_lucida_fallback_flip; restore
diff -r $O/ref.orig $REF && cmp $O/gen.orig $GEN && echo RESTORED
