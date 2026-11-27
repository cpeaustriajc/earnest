const path = require('path');
const fs = require('fs');
const dist = path.join(process.env.QUINT_PACKAGE, 'dist', 'src');
const frontend = require(path.join(dist, 'parsing', 'quintParserFrontend.js'));
const { newIdGenerator } = require(path.join(dist, 'idGenerator.js'));
const { fileSourceResolver } = require(path.join(dist, 'parsing', 'sourceResolver.js'));
const { jsonStringOfOutputStage } = require(path.join(dist, 'cliReporting.js'));
const { TypeApplicationResolver } = require(path.join(dist, 'types', 'typeApplicationResolution.js'));
const { TypeInferrer } = require(path.join(dist, 'types', 'inferrer.js'));
const { errorTreeToString } = require(path.join(dist, 'errorTree.js'));

const [file, out] = process.argv.slice(2);
const main = path.resolve(file);
const text = fs.readFileSync(main, 'utf8');
const ids = newIdGenerator();
const resolver = fileSourceResolver(new Map([[main, text]]));
const mainPath = resolver.lookupPath(path.dirname(main), path.basename(main));
const parsed = frontend.parse(ids, main, mainPath, text);
if (parsed.errors.length > 0) {
  process.exit(0);
}
const { modules, table, sourceMap } = parsed;
const applications = new TypeApplicationResolver(table);
const inferrer = new TypeInferrer(table);
const errors = [];
let types = new Map();
let failure;
try {
  for (const module of modules) {
    const [applicationErrors, declarations] = applications.resolveTypeApplications(module.declarations);
    const [typeErrors, inferred] = inferrer.inferTypes(declarations);
    for (const [id, error] of [...typeErrors, ...applicationErrors]) {
      errors.push({ explanation: `[QNT000] ${errorTreeToString(error)}`, locs: sourceMap.has(id) ? [sourceMap.get(id)] : [] });
    }
    types = inferred;
  }
} catch (error) {
  failure = String(error.message);
}
fs.writeFileSync(out, jsonStringOfOutputStage({ types, errors, failure }));
