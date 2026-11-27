const path = require('path');
const fs = require('fs');
const dist = path.join(process.env.QUINT_PACKAGE, 'dist', 'src');
const frontend = require(path.join(dist, 'parsing', 'quintParserFrontend.js'));
const { newIdGenerator } = require(path.join(dist, 'idGenerator.js'));
const { expressionToString } = require(path.join(dist, 'ir', 'IRprinting.js'));

const [file, out] = process.argv.slice(2);
const main = path.resolve(file);
const parsed = frontend.parsePhase1fromText(newIdGenerator(), fs.readFileSync(main, 'utf8'), main);
const kinds = new Set(['name', 'bool', 'int', 'str', 'app', 'lambda', 'let']);
const printed = {};
const expression = value =>
  kinds.has(value.kind) &&
  value.id !== undefined &&
  (!['bool', 'int', 'str'].includes(value.kind) || 'value' in value) &&
  (value.kind !== 'app' || 'opcode' in value);
const visit = value => {
  if (Array.isArray(value)) {
    value.forEach(visit);
  } else if (value !== null && typeof value === 'object') {
    if (expression(value) && !(value.id in printed)) {
      printed[value.id] = expressionToString(value);
    }
    Object.values(value).forEach(visit);
  }
};
visit(parsed.modules);
fs.writeFileSync(out, JSON.stringify(printed));
