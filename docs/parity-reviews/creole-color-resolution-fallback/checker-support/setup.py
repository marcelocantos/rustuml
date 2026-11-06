import pathlib,subprocess,tarfile,io,json,hashlib
S=pathlib.Path(__file__).parent
R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ae47bffd')
M=R/'docs/parity-reviews/creole-color-resolution-fallback/maker-support'
sha=lambda b:hashlib.sha256(b).hexdigest()
a=M/'measurements.tar.gz'; assert sha(a.read_bytes())=='6331cacb74bc7420736938a33ea3610eda2b9cf2ce5a5ae4cb47d3915eb8e2ba'
with tarfile.open(a) as t:
 members=t.getmembers();assert len(members)==37232
 t.extractall(S/'maker',filter='data')
print('verified archive',len(members),flush=True)
for phase,rev in [('before','bdfeba117c7af6a527fc66b0acda6d8273fa6d74'),('after','b8905e296b8865e53f3c573abaebf4adb632d751')]:
 d=S/('src-'+phase);d.mkdir(exist_ok=True)
 data=subprocess.check_output(['git','archive',rev],cwd=R)
 with tarfile.open(fileobj=io.BytesIO(data)) as t:t.extractall(d,filter='data')
 print('snapshot',phase,rev,flush=True)
(S/'logs').mkdir(exist_ok=True)
