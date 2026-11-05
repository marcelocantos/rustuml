# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Maker replay of immutable checker manifests/references; no Java launches."""
import concurrent.futures,importlib.util,json,os,sys
from pathlib import Path
S=Path(os.environ.get('TIM_MAKER_SCRATCH','/tmp/rustuml-tim4'));SUP=Path(__file__).resolve().parents[4]/'docs/parity-reviews/tim-typed-values-and-binding/checker3-support'
spec=importlib.util.spec_from_file_location('checker',SUP/'check.py');c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
c.S=S;c.ENV['JAVA_TOOL_OPTIONS']=(os.environ.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip()
phase,suite=sys.argv[1:]
rows=json.loads((SUP/('frozen-inventory.json' if suite=='frozen' else suite+'-manifest.json')).read_text())
missing=[r for r in rows if not (c.ROOT/r['source']).exists()]
rows=[r for r in rows if (c.ROOT/r['source']).exists()]
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:results=list(pool.map(lambda row:c.measure(row,phase,suite!='frozen'),rows))
c.dump(S/(phase+'-'+suite+'.json'),results);c.dump(S/(suite+'-missing.json'),missing)
print(phase,suite,c.summary(results),'missing',len(missing),flush=True)
