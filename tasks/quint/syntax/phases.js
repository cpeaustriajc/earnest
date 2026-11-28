const path = require('path');
const fs = require('fs');
const dist = path.join(process.env.QUINT_PACKAGE, 'dist', 'src');
const frontend = require(path.join(dist, 'parsing', 'quintParserFrontend.js'));
const { newIdGenerator } = require(path.join(dist, 'idGenerator.js'));
const { fileSourceResolver } = require(path.join(dist, 'parsing', 'sourceResolver.js'));
const { jsonStringOfOutputStage } = require(path.join(dist, 'cliReporting.js'));

const [file, out] = process.argv.slice(2);
const main = path.resolve(file);
const text = fs.readFileSync(main, 'utf8');
const ids = newIdGenerator();
const first = frontend.parsePhase1fromText(ids, text, main);
const resolver = fileSourceResolver(new Map([[main, text]]));
const mainPath = resolver.lookupPath(path.dirname(main), path.basename(main));
const second = frontend.parsePhase2sourceResolution(ids, resolver, mainPath, first);
const located = second.errors.map(error => ({ ...error, loc: second.sourceMap.get(error.reference) }));
const written = {
  modules: second.modules,
  errors: located,
  sourceMap: frontend.compactSourceMap(second.sourceMap),
};
fs.writeFileSync(out, jsonStringOfOutputStage(written));
