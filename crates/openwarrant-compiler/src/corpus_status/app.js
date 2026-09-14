(function(){
  'use strict';
  // The progress platform (OpenWarrant 1.0 slices D3/D4). One file, no
  // framework, no network: three JSON projections are inlined by the compiler
  // and rendered here. No HTML-string path anywhere — every string from the
  // records reaches the DOM through textContent or a text node (never the HTML-string setter). No ratio is
  // computed: every number is a count, and every count links to its rows.
  var byId=function(id){var e=document.getElementById(id); return e? JSON.parse(e.textContent):null;};
  var d=byId('corpus-status'); if(!d) return;
  var T=byId('corpus-timeline'); var P=byId('corpus-pending');
  var app=document.getElementById('app');

  // ---- tiny DOM helpers -------------------------------------------------
  function h(tag,attrs,kids){var e=document.createElement(tag); if(attrs) for(var k in attrs){ if(k==='text') e.textContent=attrs[k]; else if(k==='on') { for(var ev in attrs.on) e.addEventListener(ev,attrs.on[ev]); } else e.setAttribute(k,attrs[k]); } (kids||[]).forEach(function(c){ if(c==null) return; e.appendChild(typeof c==='string'?document.createTextNode(c):c); }); return e;}
  function tx(s){return document.createTextNode(String(s));}
  function link(href,label,cls){return h('a',{href:href,class:cls||'',text:label});}
  function table(head,rows,opts){opts=opts||{}; var t=h('table'); var tr=h('tr'); head.forEach(function(x){tr.appendChild(h('th',{text:x}));}); t.appendChild(h('thead',null,[tr])); var tb=h('tbody'); rows.forEach(function(r,i){var tr2=h('tr',{tabindex:'0','data-row':String(i)}); if(opts.href) { tr2.setAttribute('data-href',opts.href(i)); } r.forEach(function(c){tr2.appendChild(h('td',null,[typeof c==='number'?String(c):c]));}); tb.appendChild(tr2);}); t.appendChild(tb); return h('div',{class:'scroll'},[t]);}
  function count(list,pred){var n=0; list.forEach(function(x){ if(pred(x)) n++; }); return n;}
  function ref(r){return r? (r.slug? 'roadmap://'+r.prefix+'-PHASE-'+r.phase+'/'+r.slug : 'roadmap://'+r.prefix+'-PHASE-'+r.phase) : '';}
  function rq(r){return r.prefix+'-SAS-RQ-'+String(r.number).padStart(3,'0');}
  function pill(txt,cls){return h('span',{class:'pill '+cls,text:txt});}
  function kv(pairs){var dl=h('dl',{class:'kv'}); pairs.forEach(function(p){ if(p[1]==null||p[1]==='') return; dl.appendChild(h('dt',{text:p[0]})); dl.appendChild(h('dd',null,[typeof p[1]==='string'?tx(p[1]):p[1]])); }); return dl;}
  function warrantLink(alias){return link('#/warrant/'+alias,alias,'');}
  function short(s,n){s=String(s||''); return s.length>n? s.slice(0,n-1)+'…':s;}

  // ---- routing ------------------------------------------------------------
  var VIEWS=[['dashboard','Dashboard'],['objectives','Objectives'],['warrants','Warrants'],['requirements','Requirements'],['timeline','Timeline'],['gaps','Gaps'],['pending','Pending'],['evidence','Evidence']];
  function route(){var hsh=location.hash||'#/dashboard'; var m=hsh.replace(/^#\/?/,''); var q={}; var qi=m.indexOf('?'); if(qi>=0){ m.slice(qi+1).split('&').forEach(function(kv2){var p=kv2.split('='); if(p[0]) q[decodeURIComponent(p[0])]=decodeURIComponent(p[1]||'');}); m=m.slice(0,qi);} var parts=m.split('/'); return {view:parts[0]||'dashboard',arg:parts[1]||'',q:q};}
  function go(hsh){location.hash=hsh;}

  // ---- indexes ------------------------------------------------------------
  // The projection omits an empty list (docs/PROJECTION_CONTRACT.md: every
  // added field defaults); the app reads them as empty, never as absent.
  d.warrants.forEach(function(w){ ['obligations','deliverables','gate_runs','amendments','unknowns','unmet','unestablished','blocking_unknowns','roadmap','implements'].forEach(function(k){ if(!w[k]) w[k]=[]; }); });
  var W={}; d.warrants.forEach(function(w){W[w.alias]=w;});
  var RUNGS=['resolved','would_satisfy','ready_to_resolve','draft','invalid'];
  function rungOf(w){return w.validity && w.validity.state==='invalid' ? 'invalid' : w.rung;}
  var ladder={}; RUNGS.forEach(function(r){ladder[r]=count(d.warrants,function(w){return rungOf(w)===r;});});
  var pendingActs=(P&&P.acts)||[];
  var events=(T&&T.events)||[]; var days=(T&&T.days)||[];

  // ---- chrome -------------------------------------------------------------
  function chrome(current){
    var nav=h('nav',{class:'tabs',role:'tablist'}); VIEWS.forEach(function(v){ nav.appendChild(link('#/'+v[0],v[1],v[0]===current?'on':'')); });
    var theme=h('button',{class:'theme right',type:'button',text:'theme',on:{click:function(){var r=document.documentElement; var cur=r.getAttribute('data-theme'); var next=cur==='dark'?'light':cur==='light'?'':'dark'; if(next) r.setAttribute('data-theme',next); else r.removeAttribute('data-theme'); try{ if(next) localStorage.setItem('ow-theme',next); else localStorage.removeItem('ow-theme'); }catch(e){} }}});
    nav.appendChild(theme); return nav;
  }
  try{ var saved=localStorage.getItem('ow-theme'); if(saved) document.documentElement.setAttribute('data-theme',saved);}catch(e){}

  // ---- views --------------------------------------------------------------
  function viewDashboard(){
    var out=[];
    if(d.caveats && d.caveats.length){ out.push(h('h2',{text:'Read this first'})); out.push(h('ul',{class:'caveats'}, d.caveats.map(function(c){return h('li',{text:c});}))); }
    var rel=d.release; var c=rel.requirements;
    out.push(h('h2',{text:'Release'}));
    out.push(h('p',{class:'muted',text:'SAS revision: '+(rel.version||'not recorded')+'. '+rel.note}));
    out.push(h('div',{class:'rungs'},[['satisfied',c.satisfied,true],['in_progress',c.in_progress],['claimed',c.claimed],['unaddressed',c.unaddressed],['superseded',c.superseded]].map(function(x){return h('a',{class:'rung'+(x[2]?' head':''),href:'#/requirements?status='+x[0]},[h('b',{text:String(x[1])}),tx(x[0])]);})));
    out.push(h('h2',{text:'Warrants ('+d.warrants.length+')'}));
    out.push(h('div',{class:'rungs'},RUNGS.map(function(r,i){return h('a',{class:'rung'+(i===0?' head':''),href:'#/warrants?rung='+r},[h('b',{text:String(ladder[r])}),tx(r)]);})));
    out.push(h('h2',{text:'Waiting for a human ('+pendingActs.length+')'}));
    if(pendingActs.length){ out.push(h('ul',{class:'next'},pendingActs.slice(0,5).map(function(a){return h('li',null,[warrantLink(a.warrant),tx(' '+a.action+' — '),h('code',{text:a.command})]);}))); if(pendingActs.length>5) out.push(h('p',null,[link('#/pending','all '+pendingActs.length+' pending acts')])); }
    else out.push(h('p',{class:'muted',text:'Nothing awaits a signature.'}));
    out.push(h('h2',{text:'Next actionable'}));
    if(d.next_actionable.length){ out.push(h('ul',{class:'next'}, d.next_actionable.map(function(s){return h('li',null,[warrantLink(s.warrant),tx(' / '),link('#/milestones/'+s.warrant,s.milestone+' / '+s.stage),tx(' ('+ref(s.objective)+') — '+s.why)]);}))); }
    else { var n=d.nothing_actionable; out.push(h('p',{text: n? ('Nothing is unblocked. '+n.why+(n.blocked_by.length?' Blocked by: '+n.blocked_by.join(', '):'')) : 'Nothing is unblocked, and the projection could not say why. That is a defect in the projection.'})); }
    var blockers={}; d.warrants.forEach(function(w){ w.unmet.forEach(function(u){ blockers[u]=(blockers[u]||0)+1; }); });
    var bk=Object.keys(blockers).sort(function(a,b){return blockers[b]-blockers[a]||a.localeCompare(b);});
    if(bk.length){ out.push(h('h2',{text:'What blocks resolution, by §56.1 requirement'})); out.push(h('p',{class:'muted',text:'Warrants blocked on each requirement. Names, not a score: this says what to fix.'})); out.push(table(['requirement','Warrants blocked'], bk.map(function(k){return [k,link('#/warrants?unmet='+encodeURIComponent(k),String(blockers[k]))];}))); }
    return out;
  }

  function viewObjectives(){
    var out=[h('h2',{text:'Objectives (SAS §98 phases)'})];
    out.push(table(['Objective','Exit Warrant','Achieved','invalid','draft','ready','would_satisfy','resolved'], d.objectives.map(function(o){
      var a=o.achieved, at=a.state==='not_derivable'?'not derivable — '+a.why : a.state==='blocked'?'blocked by '+a.by.join(', ') : a.state==='exit_warrant_would_satisfy'?'exit Warrant would satisfy; not recorded':'recorded';
      var name=o.roadmap_ref? ref(o.roadmap_ref)+': '+o.title : o.title; var l=o.ladder; var pre='#/warrants?objective='+encodeURIComponent(o.roadmap_ref?ref(o.roadmap_ref):'unassigned');
      return [h('span',{text:name}), o.exit_warrant? warrantLink(o.exit_warrant):tx('—'), h('span',{text:at}), link(pre+'&rung=invalid',String(l.invalid)), link(pre+'&rung=draft',String(l.draft)), link(pre+'&rung=ready_to_resolve',String(l.ready_to_resolve)), link(pre+'&rung=would_satisfy',String(l.would_satisfy)), link(pre+'&rung=resolved',String(l.resolved))];
    })));
    out.push(h('ul',{class:'muted'}, d.objectives.filter(function(o){return o.exit_criterion;}).map(function(o){return h('li',null,[h('b',{text:ref(o.roadmap_ref)}),tx(' exit: '+o.exit_criterion)]);})));
    return out;
  }

  function viewWarrants(q){
    var out=[h('h2',{text:'Warrants ('+d.warrants.length+')'})];
    var tb=h('div',{class:'toolbar'});
    function sel(name,label,opts){var s=h('select',{name:name}); s.appendChild(h('option',{value:'',text:'any'})); opts.forEach(function(o){var op=h('option',{value:o,text:o}); if(q[name]===o) op.setAttribute('selected','selected'); s.appendChild(op);}); s.addEventListener('change',function(){ q[name]=s.value; if(!s.value) delete q[name]; go('#/warrants?'+Object.keys(q).map(function(k){return k+'='+encodeURIComponent(q[k]);}).join('&')); }); tb.appendChild(h('label',null,[tx(label+' '),s]));}
    sel('rung','rung',RUNGS); sel('level','assurance',['basic','controlled','high'].filter(function(x){return d.warrants.some(function(w){return w.assurance_level===x;});})); sel('profile','profile',['delivery','decision']); sel('unknowns','unknowns',['blocking','none']);
    var phases={}; d.warrants.forEach(function(w){w.roadmap.forEach(function(r){phases[ref(r)]=1;});}); sel('objective','objective',Object.keys(phases).sort().concat(['unassigned']));
    out.push(tb);
    var rows=d.warrants.filter(function(w){
      if(q.rung && rungOf(w)!==q.rung) return false;
      if(q.level && w.assurance_level!==q.level) return false;
      if(q.profile && w.profile!==q.profile) return false;
      if(q.unmet && w.unmet.indexOf(q.unmet)<0) return false;
      if(q.unknowns==='blocking' && !w.blocking_unknowns.length) return false;
      if(q.unknowns==='none' && w.blocking_unknowns.length) return false;
      if(q.objective){ var refs=w.roadmap.map(ref); if(q.objective==='unassigned'? refs.length : refs.indexOf(q.objective)<0) return false; }
      return true;
    });
    out.push(h('p',{class:'muted',text:rows.length+' shown. Filters are in the URL; share it. Keys: j/k move, Enter opens.'}));
    out.push(table(['Warrant','rung','level','§38.6','§56.1 (13)','blocking unknowns','milestones evidenced','first unmet'], rows.map(function(w){
      var r=rungOf(w); var rungTxt = r==='invalid' ? 'invalid — '+w.validity.reason : r;
      var o = w.would_resolve_satisfied===true?'would satisfy': w.would_resolve_satisfied===false?'NOT satisfied':'unknown';
      var strip=h('span',{class:'strip',title:'the thirteen §56.1 requirements'}); if(w.checks){ Object.keys(w.checks).forEach(function(k){ strip.appendChild(h('i',{class:w.checks[k]?'t':'f',title:k})); }); }
      var ev = w.milestones? (count(w.milestones,function(m){return m.reached.state==='evidenced';})+' of '+w.milestones.length) : '—';
      return [h('span',null,[warrantLink(w.alias),tx(' '+short(w.title||'',80))]), rungTxt, w.assurance_level||'—', pill(o,o==='would satisfy'?'ok':o==='unknown'?'warn':'bad'), strip, String(w.blocking_unknowns.length), ev, (w.unmet[0]||'—')];
    }),{href:function(i){return '#/warrant/'+rows[i].alias;}}));
    return out;
  }

  function viewWarrant(alias){
    var w=W[alias]; if(!w) return [h('h2',{text:'No Warrant '+alias})];
    var out=[h('h2',null,[h('code',{text:w.alias}),tx(' '+(w.title||''))])];
    var res=w.resolution;
    out.push(kv([['uuid',w.uuid],['profile / assurance',(w.profile||'')+' / '+(w.assurance_level||'')],['rung',rungOf(w)],['state',w.state? (w.state.phase||JSON.stringify(w.state)) : ''],['contract',w.contract_revision? 'revision '+w.contract_revision+' · '+short(w.contract_digest||'',20):(w.contract_digest?short(w.contract_digest,20):'')],['resolution',res? res.common_outcome+' / '+res.profile_outcome+' by '+res.resolved_by_ref+' at '+res.effective_at+(res.binds_current_contract?'':' (does NOT bind the current contract)'):''],['implements',w.implements.map(function(c){return rq(c.requirement)+' ('+c.contribution+')';}).join(', ')],['roadmap',w.roadmap.map(ref).join(', ')],['records',[w.authorization_ref,w.resolution_ref,w.journal_ref].filter(Boolean).join(' · ')],['milestones',w.milestones? h('a',{href:'#/milestones/'+w.alias,text:w.milestones.length+' milestone(s), DAG'}):'']]));
    var acts=pendingActs.filter(function(a){return a.warrant===w.alias;}); if(acts.length){ out.push(h('h3',{text:'Waiting for a human'})); acts.forEach(function(a){ out.push(h('pre',{class:'cmd',text:a.command+'   # '+a.action+': '+a.why})); }); }
    out.push(h('h3',{text:'§56.1 — the thirteen requirements'}));
    if(w.checks){ out.push(table(['requirement','met'],Object.keys(w.checks).map(function(k){return [k,pill(w.checks[k]?'true':'false',w.checks[k]?'ok':'bad')];}))); } else out.push(h('p',{class:'muted',text:'not computed (invalid record)'}));
    out.push(h('h3',{text:'Obligations ('+w.obligations.length+')'}));
    out.push(table(['id','statement','scope','gate','disposition','verifier'],w.obligations.map(function(o){return [h('code',{text:o.id}),o.statement,short(o.scope,90),o.gate? h('code',{text:o.gate.replace('gate://','')}):tx('—'),pill(o.disposition,o.disposition==='established'?'ok':o.disposition==='undispositioned'?'warn':'bad'),(o.verifier||'—')+(o.verifier_kind?' ('+o.verifier_kind+')':'')];})));
    out.push(h('h3',{text:'Deliverables ('+w.deliverables.length+')'}));
    out.push(table(['id','title','target','digest','corrections'],w.deliverables.map(function(x){return [h('code',{text:x.id}),x.title,h('code',{text:x.target_ref}),pill(x.digest,x.digest==='verified'||x.digest==='corrected'?'ok':x.digest==='not_content_addressed'?'':'bad'),String(x.corrections)];})));
    out.push(h('h3',{text:'Gate runs ('+w.gate_runs.length+')'}));
    out.push(table(['gate','run','verdict','class','why'],w.gate_runs.map(function(g){return [h('code',{text:g.gate}),h('code',{text:g.run_id}),g.verdict,pill(g.class,g.class==='admissible'?'ok':g.class==='stale_binding'?'warn':'bad'),g.why||''];})));
    if(w.amendments.length){ out.push(h('h3',{text:'Amendments ('+w.amendments.length+')'})); out.push(table(['id','reason','changes'],w.amendments.map(function(a){return [h('code',{text:a.id}),a.reason,h('ul',null,a.changes.map(function(c){return h('li',null,[h('code',{text:c[0]}),tx(': '+short(c[1],40)+' → '+short(c[2],40))]);}))];}))); }
    if(w.unknowns.length){ out.push(h('h3',{text:'Assumptions and unknowns ('+w.unknowns.length+')'})); out.push(table(['id','status','statement','mentions'],w.unknowns.map(function(u){return [h('code',{text:u.id}),pill(u.epistemic_status,u.epistemic_status==='blocking_unknown'?'bad':'warn'),u.statement,u.mentions.join(', ')];}))); }
    var evs=events.filter(function(e){return e.warrant===w.alias;}); out.push(h('h3',{text:'Journal ('+evs.length+' event(s))'}));
    out.push(table(['when','event','actor'],evs.map(function(e){return [h('span',{class:'nowrap',text:e.occurred_at}),h('code',{text:e.event_type}),e.actor_ref];})));
    return out;
  }

  function viewMilestones(alias){
    var w=W[alias]; if(!w||!w.milestones) return [h('h2',{text:'No milestones for '+alias})];
    var ms=w.milestones; var out=[h('h2',null,[tx('Milestones of '),warrantLink(alias)])];
    // Longest-path layering: a milestone's layer is one more than the deepest of its dependencies.
    var layer={}; function L(id,seen){ if(layer[id]!=null) return layer[id]; seen=seen||{}; if(seen[id]) return 0; seen[id]=1; var m=ms.filter(function(x){return x.id===id;})[0]; var deps=(m&&m.depends_on)||[]; var best=0; deps.forEach(function(x){ best=Math.max(best,L(x,seen)+1); }); layer[id]=best; return best; }
    ms.forEach(function(m){L(m.id);});
    var cols={}; ms.forEach(function(m){ (cols[layer[m.id]]=cols[layer[m.id]]||[]).push(m); });
    var W0=190,H0=54,GX=70,GY=18,PAD=16; var ncol=Object.keys(cols).length; var maxRows=0; Object.keys(cols).forEach(function(k){maxRows=Math.max(maxRows,cols[k].length);});
    var width=PAD*2+ncol*W0+(ncol-1)*GX, height=PAD*2+maxRows*H0+(maxRows-1)*GY;
    var svgNS='http://www.w3.org/2000/svg'; var svg=document.createElementNS(svgNS,'svg'); svg.setAttribute('class','dag'); svg.setAttribute('viewBox','0 0 '+width+' '+height); svg.setAttribute('role','img');
    var pos={}; Object.keys(cols).sort(function(a,b){return a-b;}).forEach(function(k){ cols[k].forEach(function(m,i){ pos[m.id]={x:PAD+k*(W0+GX), y:PAD+i*(H0+GY)}; }); });
    function el(n,attrs){var e=document.createElementNS(svgNS,n); for(var k in attrs) e.setAttribute(k,attrs[k]); return e;}
    ms.forEach(function(m){ (m.depends_on||[]).forEach(function(dep){ var a=pos[dep],b=pos[m.id]; if(!a||!b) return; var x1=a.x+W0,y1=a.y+H0*0.5,x2=b.x,y2=b.y+H0*0.5; var mid=(x1+x2)*0.5; svg.appendChild(el('path',{class:'edge','d':'M'+x1+','+y1+' C'+mid+','+y1+' '+mid+','+y2+' '+x2+','+y2})); }); });
    ms.forEach(function(m){ var p=pos[m.id]; var g=el('g',{}); g.appendChild(el('rect',{class:'box '+m.reached.state,x:p.x,y:p.y,width:W0,height:H0,rx:6})); var t1=el('text',{x:p.x+8,y:p.y+20}); t1.textContent=m.id+' · '+m.reached.state; var t2=el('text',{x:p.x+8,y:p.y+40}); t2.textContent=short(m.title||'',26); g.appendChild(t1); g.appendChild(t2); svg.appendChild(g); });
    out.push(svg);
    out.push(table(['milestone','reached','depends on','stages','obligations'],ms.map(function(m){return [h('code',{text:m.id+' '+(m.title||'')}),pill(m.reached.state,m.reached.state==='evidenced'?'ok':m.reached.state==='unblocked'?'':'warn'),(m.depends_on||[]).join(', '),(m.stages||[]).join(', '),(m.obligations||[]).join(', ')];})));
    return out;
  }

  function viewRequirements(q){
    var out=[h('h2',{text:'Requirements (SAS §106, §34.3)'})];
    var tb=h('div',{class:'toolbar'}); var s=h('select'); s.appendChild(h('option',{value:'',text:'any status'})); ['satisfied','in_progress','claimed','unaddressed','superseded'].forEach(function(o){var op=h('option',{value:o,text:o}); if(q.status===o) op.setAttribute('selected','selected'); s.appendChild(op);}); s.addEventListener('change',function(){go('#/requirements'+(s.value?'?status='+s.value:''));}); tb.appendChild(h('label',null,[tx('status '),s])); out.push(tb);
    var rows=d.requirements.filter(function(r){return !q.status||r.status===q.status;});
    out.push(h('p',{class:'muted',text:rows.length+' of '+d.requirements.length+' rows.'}));
    out.push(table(['requirement','title','status','would_satisfy','implementers'], rows.map(function(r){ return [h('code',{text:rq(r.requirement)}),r.title||'title unavailable',pill(r.status,r.status==='satisfied'?'ok':r.status==='unaddressed'?'bad':'warn'),String(r.would_satisfy),h('span',null,r.links.length? r.links.map(function(l,i){return h('span',null,[i?tx(', '):null,warrantLink(l.warrant),tx(' ('+l.intended_contribution+')')]);}):[tx('—')])]; })));
    return out;
  }

  function viewTimeline(q){
    var out=[h('h2',{text:'Timeline ('+events.length+' event(s), '+days.length+' day(s))'})];
    if(!T) return out.concat([h('p',{class:'muted',text:'CORPUS_TIMELINE.json is not inlined in this page.'})]);
    var maxN=0; days.forEach(function(x){maxN=Math.max(maxN,x.events);});
    var hist=h('div',{class:'hist'}); days.forEach(function(x){ var a=h('a',{href:'#/timeline?day='+x.date,title:x.date+': '+x.events+' event(s)',class:q.day===x.date?'on':''}); a.style.height=(maxN? Math.round(x.events*100/Math.max(maxN,1)):0)+'%'; hist.appendChild(a); }); out.push(hist);
    var tb=h('div',{class:'toolbar'}); var s=h('select'); s.appendChild(h('option',{value:'',text:'any event type'})); Object.keys(T.by_type).sort().forEach(function(k){var op=h('option',{value:k,text:k+' ('+T.by_type[k]+')'}); if(q.type===k) op.setAttribute('selected','selected'); s.appendChild(op);}); s.addEventListener('change',function(){ var qq=[]; if(q.day) qq.push('day='+q.day); if(s.value) qq.push('type='+encodeURIComponent(s.value)); go('#/timeline'+(qq.length?'?'+qq.join('&'):'')); }); tb.appendChild(h('label',null,[tx('type '),s])); if(q.day){ tb.appendChild(link('#/timeline','clear day '+q.day)); } out.push(tb);
    var rows=events.filter(function(e){ if(q.day && e.occurred_at.slice(0,10)!==q.day) return false; if(q.type && e.event_type!==q.type) return false; return true; });
    out.push(h('p',{class:'muted',text:rows.length+' shown, newest last.'}));
    out.push(table(['when','Warrant','event','class','actor'],rows.map(function(e){return [h('span',{class:'nowrap',text:e.occurred_at}),warrantLink(e.warrant),h('code',{text:e.event_type}),e['class'],e.actor_ref];})));
    return out;
  }

  function viewGaps(){
    var out=[h('h2',{text:'Gaps'})];
    var blocking=[]; d.warrants.forEach(function(w){ w.unknowns.forEach(function(u){ if(u.epistemic_status==='blocking_unknown') blocking.push({w:w.alias,u:u}); }); });
    var byDep={}; blocking.forEach(function(b){ var keys=b.u.mentions.length? b.u.mentions:['undeclared']; keys.forEach(function(k){ (byDep[k]=byDep[k]||[]).push(b); }); });
    out.push(h('h3',{text:'Blocking unknowns by external dependency ('+blocking.length+')'}));
    var keys=Object.keys(byDep).sort(function(a,b){ if(a==='undeclared') return -1; if(b==='undeclared') return 1; return a.localeCompare(b); });
    if(!keys.length) out.push(h('p',{class:'muted',text:'No assumption is marked blocking_unknown.'}));
    keys.forEach(function(k){ out.push(h('h4',{text:k+' ('+byDep[k].length+')'})); out.push(table(['Warrant','id','statement'],byDep[k].map(function(b){return [warrantLink(b.w),h('code',{text:b.u.id}),b.u.statement];}))); });
    var un=[]; d.warrants.forEach(function(w){ w.obligations.forEach(function(o){ if(o.disposition!=='established') un.push({w:w,o:o}); }); });
    out.push(h('h3',{text:'Obligations not established ('+un.length+')'}));
    var byObj={}; un.forEach(function(x){ var k=x.w.roadmap.length? ref(x.w.roadmap[0]) : 'unassigned'; (byObj[k]=byObj[k]||[]).push(x); });
    Object.keys(byObj).sort().forEach(function(k){ out.push(h('h4',{text:k+' ('+byObj[k].length+')'})); out.push(table(['Warrant','obligation','disposition','statement'],byObj[k].map(function(x){return [warrantLink(x.w.alias),h('code',{text:x.o.id}),pill(x.o.disposition,x.o.disposition==='undispositioned'?'warn':'bad'),short(x.o.statement,120)];}))); });
    var unaddr=d.requirements.filter(function(r){return r.status==='unaddressed';});
    out.push(h('h3',{text:'Requirements no Warrant names ('+unaddr.length+')'}));
    out.push(h('ul',null,unaddr.map(function(r){return h('li',null,[h('code',{text:rq(r.requirement)}),tx(' — '+(r.title||'title unavailable'))]);})));
    var drift=[]; d.warrants.forEach(function(w){ w.deliverables.forEach(function(x){ if(x.digest==='drift'||x.digest==='target_unreadable') drift.push({w:w.alias,x:x}); }); });
    out.push(h('h3',{text:'Deliverables whose bytes moved ('+drift.length+')'}));
    if(drift.length) out.push(table(['Warrant','deliverable','target','state'],drift.map(function(y){return [warrantLink(y.w),h('code',{text:y.x.id}),h('code',{text:y.x.target_ref}),pill(y.x.digest,'bad')];}))); else out.push(h('p',{class:'muted',text:'None.'}));
    return out;
  }

  function viewPending(){
    var out=[h('h2',{text:'Pending human acts ('+pendingActs.length+')'})];
    if(!P) return out.concat([h('p',{class:'muted',text:'CORPUS_PENDING.json is not inlined in this page.'})]);
    if(!pendingActs.length) out.push(h('p',{class:'muted',text:'Nothing awaits a signature.'}));
    pendingActs.forEach(function(a){ var pre=h('pre',{class:'cmd',text:a.command}); var btn=h('button',{class:'linkish tiny',type:'button',text:'copy',on:{click:function(){ try{ navigator.clipboard.writeText(a.command); btn.textContent='copied'; }catch(e){ btn.textContent='select and copy'; } }}}); out.push(h('div',{class:'card'},[h('h3',null,[warrantLink(a.warrant),tx(' · '+a.action)]),h('p',{class:'muted',text:a.why}),pre,btn])); });
    return out;
  }

  function viewEvidence(){
    var runs=[]; d.warrants.forEach(function(w){ w.gate_runs.forEach(function(g){ runs.push({w:w.alias,g:g}); }); });
    var out=[h('h2',{text:'Evidence — every committed gate run ('+runs.length+')'})];
    var byClass={}; runs.forEach(function(r){ byClass[r.g['class']]=(byClass[r.g['class']]||0)+1; });
    out.push(h('div',{class:'rungs'},Object.keys(byClass).sort().map(function(k){return h('span',{class:'rung'+(k==='admissible'?' head':'')},[h('b',{text:String(byClass[k])}),tx(k)]);})));
    out.push(table(['Warrant','gate','run','verdict','class','receipt','why'],runs.map(function(r){return [warrantLink(r.w),h('code',{text:r.g.gate}),h('code',{text:short(r.g.run_id,24)}),r.g.verdict,pill(r.g['class'],r.g['class']==='admissible'?'ok':r.g['class']==='stale_binding'?'warn':'bad'),r.g.receipt_ref? h('code',{text:short(r.g.receipt_ref,48)}):tx('—'),r.g.why||''];})));
    return out;
  }

  // ---- render ---------------------------------------------------------------
  function render(){
    var r=route(); var view=r.view; var body;
    switch(view){
      case 'objectives': body=viewObjectives(); break;
      case 'warrants': body=viewWarrants(r.q); break;
      case 'warrant': body=viewWarrant(r.arg); view='warrants'; break;
      case 'milestones': body=viewMilestones(r.arg); view='warrants'; break;
      case 'requirements': body=viewRequirements(r.q); break;
      case 'timeline': body=viewTimeline(r.q); break;
      case 'gaps': body=viewGaps(); break;
      case 'pending': body=viewPending(); break;
      case 'evidence': body=viewEvidence(); break;
      default: body=viewDashboard(); view='dashboard';
    }
    while(app.firstChild) app.removeChild(app.firstChild);
    app.appendChild(chrome(view));
    body.forEach(function(x){app.appendChild(x);});
    var f=h('footer',null,[h('h2',{text:'Not reported here'}),h('ul',null,[
      h('li',{text:'Any percentage. Every number above is a rung count or a histogram of names, and every count links to its rows.'}),
      h('li',{text:'Whether a §98 Exit sentence is true. It is printed; it is not evaluated.'}),
      h('li',{text:'Anything outside the committed records: this page inlines three projections and fetches nothing.'})])]);
    app.appendChild(f);
    window.scrollTo(0,0);
  }
  // Keyboard: j/k move between rows of the focused table, Enter opens a row that links somewhere.
  document.addEventListener('keydown',function(ev){
    if(ev.target && (ev.target.tagName==='INPUT'||ev.target.tagName==='SELECT'||ev.target.tagName==='TEXTAREA')) return;
    var rows=Array.prototype.slice.call(app.querySelectorAll('tbody tr')); if(!rows.length) return;
    var cur=rows.indexOf(document.activeElement); var next=cur;
    if(ev.key==='j') next=Math.min(rows.length-1,cur+1); else if(ev.key==='k') next=Math.max(0,cur-1);
    else if(ev.key==='Enter' && cur>=0){ var hr=rows[cur].getAttribute('data-href'); if(hr) go(hr); return; }
    else return;
    rows.forEach(function(x){x.classList.remove('sel');}); rows[next].classList.add('sel'); rows[next].focus(); ev.preventDefault();
  });
  window.addEventListener('hashchange',render);
  render();
})();
