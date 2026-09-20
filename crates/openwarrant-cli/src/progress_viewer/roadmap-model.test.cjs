// SPDX-License-Identifier: Apache-2.0
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {readFileSync}=require('node:fs');
const vm=require('node:vm');
const model=vm.createContext({});
vm.runInContext(readFileSync(__dirname+'/roadmap-model.js','utf8'),model);
const nodes=[
 {id:'release',title:'Release',outcome:'Ship',warrants:[]},
 {id:'a',parent:'release',title:'Parser',outcome:'Readable sources',warrants:['W1','W2']},
 {id:'b',parent:'release',title:'SDK',outcome:'API',warrants:['W1']}
];
const entries={W1:{report:{work_state:'completed'}},W2:{report:{work_state:'blocked'}}};
test('shared references count once; blocked and missing reports do not count complete',()=>{
 const s=model.roadmapStats(nodes,'release',entries);
 assert.equal(s.aliases.length,2);assert.equal(s.completed,1);assert.equal(s.percent,50);
 const missing=model.roadmapStats(nodes,'release',{W1:entries.W1});
 assert.equal(missing.percent,50);assert.equal(model.roadmapMatches(missing,'','unknown'),true);
});
test('unscoped and empty groups cannot display 100 percent',()=>{
 const extended=[...nodes,{id:'future',parent:'release',title:'Future',outcome:'Later',warrants:[]}];
 const s=model.roadmapStats(extended,'release',entries);
 assert.equal(s.unscoped,1);assert.equal(s.percent,null);
 assert.equal(model.roadmapStats(extended,'future',entries).percent,null);
});
test('query and status filters retain only matching branches',()=>{
 const s=model.roadmapStats(nodes,'release',entries);
 assert.equal(model.roadmapMatches(s,'PARSER','active'),true);
 assert.equal(model.roadmapMatches(s,'','blocked'),true);
 assert.equal(model.roadmapMatches(model.roadmapStats(nodes,'b',entries),'','blocked'),false);
 assert.equal(model.roadmapMatches(s,'missing','active'),false);
 assert.equal(model.roadmapMatches(model.roadmapStats(nodes,'b',entries),'','active'),false);
 assert.equal(model.roadmapMatches(model.roadmapStats(nodes,'b',entries),'W1','completed'),true);
});

test('percentage cannot reach 100 while a referenced Warrant lacks completion',()=>{
 const warrants=Array.from({length:201},(_,i)=>'W'+i);
 const groups=[{id:'release',title:'Release',outcome:'Ship',warrants}];
 const reports=Object.fromEntries(warrants.slice(0,-1).map(id=>[id,{report:{work_state:'completed'}}]));
 const partial=model.roadmapStats(groups,'release',reports);
 assert.equal(partial.completed,200);assert.equal(partial.percent,99);
 reports.W200={report:{work_state:'completed'}};
 assert.equal(model.roadmapStats(groups,'release',reports).percent,100);
});
