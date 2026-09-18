// SPDX-License-Identifier: Apache-2.0
// Pure calculations shared by the embedded renderer and Node's built-in tests.
function roadmapMembers(nodes, id) {
 const selected=[],pending=[id];
 while(pending.length){const next=pending.pop();const node=nodes.find(n=>n.id===next);if(!node)continue;selected.push(node);for(const child of nodes.filter(n=>n.parent===next))pending.push(child.id);}
 return selected;
}
function roadmapStats(nodes, id, entries) {
 const members=roadmapMembers(nodes,id),aliases=[...new Set(members.flatMap(n=>n.warrants))];
 const reports=aliases.map(alias=>entries[alias]?.report);
 const completed=reports.filter(r=>r?.work_state==='completed').length;
 const unscoped=members.filter(n=>!n.warrants.length&&!nodes.some(c=>c.parent===n.id)).length;
 return {members,aliases,reports,completed,unscoped,percent:aliases.length&&!unscoped?Math.floor(completed*100/aliases.length):null};
}
function roadmapMatches(stats, query, filter) {
 const {members,reports,completed,unscoped}=stats;
 const state=filter==='all'||filter==='completed'&&completed>0||filter==='reported'&&reports.some(Boolean)||filter==='blocked'&&reports.some(r=>r?.work_state==='blocked')||filter==='unknown'&&(reports.some(r=>!r)||unscoped>0)||filter==='active'&&reports.some(r=>['in-progress','blocked'].includes(r?.work_state));
 return Boolean(state&&members.some(n=>(n.title+' '+n.outcome+' '+n.warrants.join(' ')).toLowerCase().includes(query.toLowerCase())));
}
