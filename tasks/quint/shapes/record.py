import re,subprocess,json,glob,os,sys
Q='/tmp/claude-0/q/node_modules/.bin/quint'
os.makedirs('fixtures',exist_ok=True)
here=os.getcwd()
table={}
for f in sorted(glob.glob('*.qnt')):
    n=f[:-4]
    if n=='far': continue
    subprocess.run([Q,'parse','--out','/tmp/claude-0/q/sp.json','--source-map','/tmp/claude-0/q/sm.json',f],check=True,capture_output=True)
    open(f'fixtures/{n}.parse.json','w').write(open('/tmp/claude-0/q/sp.json').read().replace(here+'/',''))
    open(f'fixtures/{n}.sourcemap.json','w').write(open('/tmp/claude-0/q/sm.json').read().replace(here+'/',''))
    text=open(f).read()
    vals=set(re.findall(r'val (\w+) =',text))|({'farOk'} if 'Far' in text else set())
    names=set(re.findall(r'module (\w+)',text))|set(re.findall(r' as (\w+)',text))|({'Far'} if 'Far' in text else set())
    names.discard('U')
    prefixes=['']+[p+'::' for p in names]+[a+'::'+b+'::' for a in names for b in names if a!=b]
    accepted=[]
    for v in sorted(vals):
        for p in prefixes:
            c=p+v
            r=subprocess.run([Q,'compile','--target','json','--main','U','--init','uInit','--step','uStep','--invariant',c,f],capture_output=True,text=True)
            if r.returncode==0: accepted.append(c)
    table[n]=sorted(accepted)
    print(n,table[n],flush=True)
json.dump(table,open('fixtures/accepted.json','w'),indent=2,sort_keys=True)
open('fixtures/accepted.json','a').write('\n')
