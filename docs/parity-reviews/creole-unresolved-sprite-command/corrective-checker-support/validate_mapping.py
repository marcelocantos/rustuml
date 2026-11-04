# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,json,gzip,base64,hashlib
S=pathlib.Path(__file__).parent;sha=lambda b:hashlib.sha256(b).hexdigest()
expected={'input-bundle.json.gz':'5a76331dd6f7f8759f19d0330b3a0eb2aa43142702fbb3fcd90311b1da5bc0c8','parent-relay-input.json.gz':'cbd90ddb2d68b5048bcfac3b44e59e321ff4cb6947394e08e1b06ece28eb967f','reference-results.json.gz':'275dcf6c83a99246c947016edafc7f92e4d8b9c4c154961f69e7f106dea78c45','reference-transport.json':'765d3014b68efc9baa5682527a0c8c4cccfb56ffeae33da559c268462c7396db'}
for name,h in expected.items():assert sha((S/name).read_bytes())==h,name
v=json.loads(gzip.decompress((S/'parent-relay-input.json.gz').read_bytes()));o=json.loads(gzip.decompress((S/'input-bundle.json.gz').read_bytes()))
for key in ['request','manifest']:
 assert base64.b64decode(v[key+'_base64'],validate=True)==(S/(key+'.json')).read_bytes()
 assert v[key]==o[key]==json.loads((S/(key+'.json')).read_bytes())
assert v['original_input_bundle_sha256']==expected['input-bundle.json.gz'];assert len(v['resources'])==len(o['files'])==26
seen=set()
for r,f in zip(v['resources'],o['files']):
 p=pathlib.PurePosixPath(r['remote_relative_path']);assert len(p.parts)==2 and p.parts[0]=='new' and p.suffix=='.puml' and '..' not in p.parts and not p.is_absolute();assert str(p) not in seen;seen.add(str(p))
 b=base64.b64decode(r['data'],validate=True);assert b==base64.b64decode(f['base64'],validate=True)==pathlib.Path(r['local_path']).read_bytes()
 for key in ['local_path','remote_relative_path','sha256','bytes']:assert r[key]==f[key]
 assert len(b)==r['bytes'] and sha(b)==r['sha256']
(S/'independent-mapping-validation.json').write_text(json.dumps(dict(all_checks_passed=True,hashes=expected,resources=26,original_request_manifest_bytes_preserved=True,safe_unique_relative_paths=True),indent=2)+'\n')
print('independent mapping and transport hashes verified')
