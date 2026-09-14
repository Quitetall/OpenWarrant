// Render every route of the progress platform against a stub DOM, so a view
// that throws on the committed projections fails the battery instead of a
// reader's browser. Node only; no dependencies. Usage:
//   node render-routes.js <CORPUS_STATUS.html> <app.js>
'use strict';
const fs = require('fs');
const vm = require('vm');
// The script under test is the repository's own committed app.js, read from
// the path this harness was handed — it is the code being exercised, not data.
const [html_path, app_path] = process.argv.slice(2);
function El(tag) { this.tagName = tag.toUpperCase(); this.children = []; this.attrs = {}; this.style = {}; this.classList = { add() {}, remove() {} }; }
El.prototype.appendChild = function (c) { this.children.push(c); return c; };
El.prototype.removeChild = function (c) { this.children = this.children.filter((x) => x !== c); };
El.prototype.setAttribute = function (k, v) { this.attrs[k] = v; };
El.prototype.getAttribute = function (k) { return this.attrs[k] || null; };
El.prototype.removeAttribute = function (k) { delete this.attrs[k]; };
El.prototype.addEventListener = function () {};
El.prototype.querySelectorAll = function () { return []; };
El.prototype.focus = function () {};
Object.defineProperty(El.prototype, 'firstChild', { get() { return this.children[0] || null; } });
Object.defineProperty(El.prototype, 'textContent', { set(v) { this._t = String(v); }, get() { return this._t || ''; } });
const html = fs.readFileSync(html_path, 'utf8');
function blob(id) {
  const m = html.match(new RegExp('<script id="' + id + '" type="application/json">([\\s\\S]*?)</script>'));
  return m ? m[1].replace(/<\\\//g, '</') : null;
}
const app = new El('div');
const store = {};
global.document = {
  getElementById(id) { if (id === 'app') return app; const b = blob(id); if (!b) return null; const e = new El('script'); e.textContent = b; return e; },
  createElement(t) { return new El(t); },
  createElementNS(ns, t) { return new El(t); },
  createTextNode(s) { return { text: String(s) }; },
  documentElement: new El('html'),
  activeElement: null,
  addEventListener() {},
};
global.window = { addEventListener() {}, scrollTo() {} };
global.location = { hash: '#/dashboard' };
global.localStorage = { getItem(k) { return store[k] || null; }, setItem(k, v) { store[k] = v; }, removeItem(k) { delete store[k]; } };
// Node 22+ defines a read-only `navigator`; define over it.
Object.defineProperty(global, 'navigator', { value: { clipboard: { writeText() {} } }, configurable: true });
const src = fs.readFileSync(app_path, 'utf8');
const status = JSON.parse(blob('corpus-status'));
const some = status.warrants[0].alias;
const withMs = (status.warrants.find((w) => w.milestones && w.milestones.length > 1) || status.warrants[0]).alias;
const routes = [
  '#/dashboard', '#/objectives', '#/warrants', '#/warrants?rung=resolved&level=basic&unknowns=blocking',
  '#/warrant/' + some, '#/milestones/' + withMs, '#/requirements?status=unaddressed', '#/timeline',
  '#/timeline?day=' + (status.warrants.length ? '2026-09-02' : '') + '&type=verification.recorded', '#/gaps', '#/pending', '#/evidence',
  '#/warrant/NOPE', '#/milestones/NOPE', '#/nonsense',
];
let failed = 0;
for (const r of routes) {
  global.location.hash = r;
  app.children = [];
  try {
    vm.runInThisContext(src);
    if (app.children.length < 2) throw new Error('rendered ' + app.children.length + ' node(s)');
    console.log('ok   ' + r + ' (' + app.children.length + ' nodes)');
  } catch (e) {
    failed++;
    console.log('FAIL ' + r + ': ' + String(e.stack || e).split('\n').slice(0, 2).join(' | '));
  }
}
console.log(routes.length - failed + '/' + routes.length + ' routes rendered');
process.exit(failed ? 1 : 0);
