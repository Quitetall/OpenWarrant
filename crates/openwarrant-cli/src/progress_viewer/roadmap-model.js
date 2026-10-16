// SPDX-License-Identifier: Apache-2.0
// Pure calculations shared by the embedded renderer and Node's built-in tests.
function roadmapMembers(nodes, id) {
 const selected=[],pending=[id];
 while(pending.length){const next=pending.pop();const node=nodes.find(n=>n.id===next);if(!node)continue;selected.push(node);for(const child of nodes.filter(n=>n.parent===next))pending.push(child.id);}
 return selected;
}
function workingFormState(record) {
 return {open:'open',in_progress:'in-progress',done:'completed'}[record?.state];
}
function roadmapStats(nodes, id, entries, forms=[]) {
 const members=roadmapMembers(nodes,id),aliases=[...new Set(members.flatMap(n=>n.warrants))];
 const reports=aliases.map(alias=>entries[alias]?.report);
 const tracked=new Map(forms.map(record=>[record.id,record]));
 const states=aliases.map(alias=>tracked.has(alias)?workingFormState(tracked.get(alias)):entries[alias]?.report?.work_state);
 const completed=states.filter(state=>state==='completed').length;
 const unscoped=members.filter(n=>!n.warrants.length&&!nodes.some(c=>c.parent===n.id)).length;
 return {members,aliases,reports,states,completed,unscoped,percent:aliases.length&&!unscoped?Math.floor(completed*100/aliases.length):null};
}
function roadmapMatches(stats, query, filter) {
 const {members,reports,states,completed,unscoped}=stats;
 const state=filter==='all'||filter==='completed'&&completed>0||filter==='reported'&&reports.some(Boolean)||filter==='blocked'&&states.includes('blocked')||filter==='open'&&states.includes('open')||filter==='unknown'&&(states.some(s=>!s)||unscoped>0)||filter==='active'&&states.some(s=>['in-progress','blocked'].includes(s));
 return Boolean(state&&members.some(n=>(n.title+' '+n.outcome+' '+n.warrants.join(' ')).toLowerCase().includes(query.toLowerCase())));
}
