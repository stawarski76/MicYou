// Read public lockfile coordinates without installing or executing dependencies.
// Usage: node docs/security/scan-lockfiles.mjs [--online]
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const rustText = readFileSync('tauri-app/Cargo.lock', 'utf8');
const bunText = readFileSync('tauri-app/bun.lock', 'utf8');
const queries = [];
const exclusions = [];
for (const block of rustText.split('[[package]]').slice(1)) {
  const name = /^name = "([^"]+)"/m.exec(block)?.[1];
  const version = /^version = "([^"]+)"/m.exec(block)?.[1];
  const source = /^source = "([^"]+)"/m.exec(block)?.[1];
  if (source === 'registry+https://github.com/rust-lang/crates.io-index' || source === 'registry+sparse+https://index.crates.io/') queries.push({package:{name, ecosystem:'crates.io'}, version});
  else exclusions.push({name, version, source:source ?? 'workspace'});
}
// Bun stores each package as a one-line array whose first string is name@version.
// Parse only that JSON string; never evaluate the lockfile as JavaScript.
// This is an inventory parser for this lockfile format, not a general JSONC/TOML
// parser. Future format changes require checking package counts for omissions.
for (const line of bunText.split('\n')) {
  const match = /^\s*"[^"\n]+":\s*\[\s*("(?:[^"\\]|\\.)*")/.exec(line);
  if (!match) continue;
  const coordinate = JSON.parse(match[1]);
  const at = coordinate.lastIndexOf('@');
  if (at < 1) continue;
  const name = coordinate.slice(0, at), version = coordinate.slice(at + 1);
  if (/^\d+\.\d+\.\d+(?:[-+][\w.-]+)?$/.test(version)) {
    queries.push({package:{name, ecosystem:'npm'}, version});
  } else exclusions.push({name, version, source:'non-registry npm coordinate'});
}
const unique = [...new Map(queries.map(q => [JSON.stringify(q), q])).values()];
const report = {
  generatedAt:new Date().toISOString(),
  scope:'Static coordinates from Cargo.lock and bun.lock, including platform/build/dev dependencies. No reachability analysis; excludes Gradle, git dependencies, vendored binaries and installed artifacts.',
  hashes:{cargo:createHash('sha256').update(rustText).digest('hex'), bun:createHash('sha256').update(bunText).digest('hex')},
  counts:Object.fromEntries(['crates.io','npm'].map(e => [e,unique.filter(q=>q.package.ecosystem===e).length])),
  exclusions,
  online:process.argv.includes('--online'),
  matches:[],
};
if (report.online) {
  for (let i=0;i<unique.length;i+=100) {
    const batch=unique.slice(i,i+100);
    const response=await fetch('https://api.osv.dev/v1/querybatch', {
      method:'POST', headers:{'Content-Type':'application/json'},
      body:JSON.stringify({queries:batch}), signal:AbortSignal.timeout(45000),
    });
    if (!response.ok) throw new Error(`OSV returned HTTP ${response.status}`);
    const data=await response.json();
    if (!Array.isArray(data.results) || data.results.length!==batch.length) throw new Error('Incomplete OSV batch response');
    for (let j=0;j<batch.length;j++) {
      if (data.results[j].vulns?.length) report.matches.push({...batch[j],vulns:data.results[j].vulns});
    }
  }
  // A lockfile match is not itself proof the vulnerable code is reachable.
  const ids=[...new Set(report.matches.flatMap(m=>m.vulns.map(v=>v.id)))];
  report.advisories=[];
  for (const id of ids) {
    const response=await fetch(`https://api.osv.dev/v1/vulns/${encodeURIComponent(id)}`,{signal:AbortSignal.timeout(45000)});
    if(!response.ok) throw new Error(`OSV advisory ${id}: HTTP ${response.status}`);
    report.advisories.push(await response.json());
  }
}
writeFileSync(`docs/security/dependencies-${report.online?'osv':'inventory'}.json`,JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({online:report.online,counts:report.counts,matches:report.matches.map(m=>({...m,vulns:m.vulns.map(v=>v.id)})),excluded:exclusions.length},null,2));
