// war ui — the page. Every value from the server is written with
// textContent, never parsed as HTML, so a record cannot inject markup. The
// session token arrives in the URL fragment, moves into this closure, and
// the fragment is cleared: it is never in a cookie, storage or a request
// URL. Acts send a row id; the server decides what runs.
//
// On the LAN (`war ui --lan`, https) there is no token: a paired device
// carries an HttpOnly cookie this script never sees, every act also sends a
// single-use nonce from the server, and a signing row shows only its verdict
// and the command to run at the host — this device can ask, never sign.
//
// Tickets (t-67ed): the first page. On loopback an open item carries Claim
// and Done, which POST {act, target[, note]} to /api/ticket; the server runs
// the same ticket commands the CLI does. On the LAN the page shows no ticket
// buttons, and the server refuses the route act.host-only regardless.
"use strict";
(() => {
  const params = new URLSearchParams(location.hash.slice(1));
  const token = params.get("t") || "";
  const lan = location.protocol === "https:";
  const pairCode = params.get("pair") || "";
  const pairFp = params.get("fp") || "";
  let page = params.get("p") || "tickets";
  history.replaceState(null, "", location.pathname + "#p=" + page);
  let version = null;

  const $ = (id) => document.getElementById(id);
  const el = (tag, attrs, ...kids) => {
    const n = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs || {})) {
      if (k === "class") n.className = v;
      else if (k === "text") n.textContent = v;
      else n.setAttribute(k, v);
    }
    for (const k of kids) if (k != null) n.append(k instanceof Node ? k : document.createTextNode(String(k)));
    return n;
  };
  const code = (s) => el("code", { text: s });
  // Tickets leads the navigation. The link is added here rather than in
  // index.html so the loopback page's bytes stay those OW-WAR-0139 OBL-005
  // pins; the page is the same either way.
  $("nav").prepend(el("a", { href: "#p=tickets", "data-page": "tickets", text: "Tickets" }));
  const api = async (path, init) => {
    const r = await fetch("/api/" + path, {
      ...init,
      headers: lan ? { ...(init && init.headers) } : { Authorization: "Bearer " + token, ...(init && init.headers) },
      cache: "no-store",
      credentials: lan ? "same-origin" : "omit",
    });
    if (r.status === 401) throw new Error(lan
      ? ((await r.text()) || "this device is not paired") + " — open a pairing link printed by `war ui --lan` at the host"
      : "session token missing or wrong — open the link `war ui` printed");
    if (!r.ok && r.status !== 202) throw new Error((await r.text()) || r.statusText);
    return r.json();
  };
  const rungBadge = (rung) => {
    const cls = rung === "resolved" ? "ok" : rung === "invalid" ? "bad" : rung === "draft" ? "warn" : "";
    return el("span", { class: "badge " + cls, text: rung || "?" });
  };
  const table = (head, rows) =>
    el("div", { class: "table-wrap" },
      el("table", {},
        el("thead", {}, el("tr", {}, ...head.map((h) => el("th", { text: h })))),
        el("tbody", {}, ...rows.map((cells) => el("tr", {}, ...cells.map((c) => el("td", {}, c)))))));

  const pages = {
    async tickets(main) {
      const d = await api("tickets");
      const list = d.tickets || [];
      main.append(el("h2", { text: "Tickets" }),
        el("p", { class: "muted", text: lan
          ? "Read-only on this device. Claim and finish tickets at the host (`war claim`, `war done`), or on its own `war ui` page."
          : "The ticket loop: claim an item, do it, mark it done. Nothing here needs a signature." }));
      if (!list.length) { main.append(el("p", { class: "muted" }, "No tickets yet — ", code("war create \"what this work accomplishes\" --item \"...\""))); return; }
      main.append(table(["ticket", "state", "done", "ticks", "p", "title", "claimed by"], list.map((t) => {
        const r = t.ticket || {};
        return [code(r.id), el("span", { class: "badge " + (r.state === "done" ? "ok" : r.state === "in_progress" ? "warn" : ""), text: String(r.state || "").replace("_", " ") }),
          `${r.done}/${r.total}`, tickSummary(r.ticks), "p" + r.priority, r.title || "", (r.claims || []).map((c) => c.actor).join(", ")];
      })));
      for (const t of list) {
        const r = t.ticket || {};
        const box = el("section", { class: "phase" });
        box.append(el("div", { class: "head" }, el("strong", { text: r.id }), el("span", { text: r.title || "" }),
          el("span", { class: "badge", text: `${r.done}/${r.total} done` })));
        const items = t.items || [];
        if (!items.length) box.append(el("p", { class: "muted" }, "No items: the ticket is the work. ",
          lan || r.state === "done" ? null : ticketButton("claim", r.id, "Claim"), " ", lan || r.state === "done" ? null : ticketButton("done", r.id, "Done")));
        else box.append(table(["", "item", "", "command"], items.map((i) => [
          i.done ? el("span", {}, "☑ ", tickBadge(i)) : (i.minimum ? el("span", {}, "☐ ", el("span", { class: "badge", text: "needs " + i.minimum, title: "ticks at " + i.minimum + " or above" })) : "☐"),
          el("span", {}, i.text, i.id ? el("span", { class: "muted", text: " (" + i.id + ")" }) : null,
            i.done && i.done_by ? el("span", { class: "muted", text: " — done by " + i.done_by + (i.note ? ": " + i.note : "") }) : null),
          i.done || lan ? (i.ready ? el("span", { class: "badge", text: "ready" }) : "")
            : el("span", {}, i.ready ? ticketButton("claim", i.target, "Claim") : null, " ", ticketButton("done", i.target, "Done")),
          i.done ? "" : code("war claim " + i.target)])));
        if ((t.notes || []).length) box.append(el("h3", { text: "Notes" }), el("ul", {}, ...t.notes.map((n) => el("li", { text: n }))));
        main.append(box);
      }
    },
    async progress(main) {
      const d = await api("progress");
      const rm = d.roadmap || {};
      main.append(el("h2", { text: (rm.program || "Program") + " — roadmap" }),
        el("p", { class: "muted", text: rm.error ? rm.error
          : rm.accepted ? "revision " + rm.accepted_revision + " accepted"
          : "not an accepted revision" + (rm.pending_revision ? " — revision " + rm.pending_revision + " awaits one signature (Queue)" : "") }));
      const L = d.ladder || {};
      const total = (L.invalid || 0) + (L.draft || 0) + (L.ready_to_resolve || 0) + (L.would_satisfy || 0) + (L.resolved || 0);
      if (total) {
        const bar = el("div", { class: "bar", title: "resolved / ready / draft" });
        for (const [k, cls] of [["resolved", "resolved"], ["ready_to_resolve", "ready"], ["would_satisfy", "ready"], ["draft", "draft"]]) {
          const s = el("span", { class: cls }); s.style.width = ((L[k] || 0) * 100 / total) + "%"; bar.append(s);
        }
        main.append(bar, el("p", { class: "muted", text: `${L.resolved || 0} resolved · ${(L.ready_to_resolve || 0) + (L.would_satisfy || 0)} ready to resolve · ${L.draft || 0} draft · ${L.invalid || 0} invalid, of ${total}` }));
      }
      for (const p of d.phases || []) {
        const box = el("section", { class: "phase" });
        const achieved = String(p.achieved || "");
        box.append(el("div", { class: "head" },
          el("strong", { text: p.id }), el("span", { text: p.title }),
          p.tier ? el("span", { class: "badge", text: "tier " + p.tier }) : null,
          el("span", { class: "badge " + (achieved === "achieved" ? "ok" : achieved.startsWith("blocked") ? "warn" : ""), text: achieved })));
        box.append(el("p", { class: "muted" }, "exit: ", p.exit || "—"));
        if ((p.depends_on || []).length) box.append(el("p", { class: "muted", text: "after " + p.depends_on.join(", ") }));
        if ((p.members || []).length)
          box.append(table(["Warrant", "title", "rung", "unmet"],
            p.members.map((m) => [code(m.alias), m.title || "", rungBadge(m.rung), String(m.unmet ?? "")])));
        if ((p.open || []).length) box.append(el("p", {}, el("span", { class: "muted", text: "no Warrant yet: " }), p.open.join(", ")));
        main.append(box);
      }
      if ((d.unassigned || []).length)
        main.append(el("h2", { text: "No phase" }),
          table(["Warrant", "title", "rung"], d.unassigned.map((m) => [code(m.alias), m.title || "", rungBadge(m.rung)])));
      // A ticket's milestones tick their marker here, each with how its tick
      // was earned: claimed < observed < independent < signed.
      if ((d.milestones || []).length)
        main.append(el("h2", { text: "Milestones" }),
          el("p", { class: "muted", text: "claimed < observed < independent < signed; a claimed tick is the performer's word, nothing checked it." }),
          table(["", "milestone", "ticket", "minimum", "tick"], d.milestones.map((m) => [
            m.met ? "☑" : "☐", el("span", {}, code(m.target), " ", m.text), m.ticket_title || "", m.minimum,
            m.level ? el("span", { class: tickClass(m.level, m.met), "data-tick": m.level, text: m.marker }) : el("span", { class: "muted", text: "open" })])));
    },
    async queue(main) {
      const d = await api("queue");
      main.append(el("h2", { text: "Awaiting a signature" }),
        el("p", { class: "note", text: lan
          ? "This device cannot sign. Each act shows its dry-run verdict and the command to run at the host; \"Ask at the host\" marks it requested there. The signature is the host's key dialog, answered at the host."
          : "A button runs the same `war sign <target> --ssh-sign` a terminal would. Your key's confirm dialog is the signature — load the key with `ssh-add -c`, or a click signs without asking." }));
      if (d.who) main.append(el("p", { class: "note" }, d.who.why, " — restart with ", code(d.who.command)));
      if (d.signer) main.append(el("p", { class: "muted" }, "signing as ", code(d.signer)));
      if (d.batch && d.batch.host_only) main.append(el("p", {}, el("span", { class: "badge", text: "at the host" }), " ", code(d.batch.command)));
      else if (d.batch) main.append(el("p", {}, actButton(d.batch.act_id, "Sign these " + d.batch.targets.length + " in one dialog"),
        " ", el("span", { class: "muted" }, "one signature over the list — ", code(d.batch.command), ". An act for another signer or role is left out and named.")));
      if (!(d.acts || []).length) main.append(el("p", { class: "muted", text: "Nothing awaits a signature." }));
      else main.append(table(["#", "act", "dry run", "", "command"], d.acts.map((a) => [
        String(a.n), el("span", {}, a.act, " ", code(a.target)),
        el("span", { class: "badge " + (a.verdict === "would record" ? "ok" : a.verdict === "needs a decision" ? "warn" : "bad"), text: a.verdict, title: (a.reasons || []).join("\n") }),
        el("span", {}, a.requested_from ? el("span", { class: "badge warn", text: "requested from " + a.requested_from }) : null,
          a.host_only ? (a.requested_from ? null : requestButton(a.request_id))
          : a.act_id ? actButton(a.act_id, "Sign")
          : (a.choices || []).length ? el("span", {}, ...a.choices.map((c) => actButton(c.act_id, c.label)))
          : el("span", { class: "muted", text: (a.reasons || [])[0] || "" })),
        code(a.command)])));
    },
    async questions(main) {
      const d = await api("questions");
      main.append(el("h2", { text: "Open questions" }));
      if (!(d.questions || []).length) main.append(el("p", { class: "muted", text: "No open questions." }));
      else main.append(table(["", "question", "recommended", "command"], d.questions.map((q) => [
        q.blocking ? el("span", { class: "badge warn", text: "blocking" }) : "",
        el("span", {}, code(q.warrant + "/" + q.id), " ", q.question), q.recommended || "", code(q.command)])));
    },
    async frontier(main) {
      const d = await api("frontier");
      main.append(el("h2", { text: `Stages — ${d.open} open · ${d.claimed} claimed · ${d.done} done · ${d.blocked} blocked` }),
        table(["state", "stage", "title", "executor", "waits on"], (d.rows || []).map((r) => [
          el("span", { class: "badge", text: r.state }), code(r.warrant + " " + r.stage), r.title, r.executor_kind, (r.waiting_on || []).join(", ")])));
    },
    async corpus(main) {
      const [d, snap] = await Promise.all([api("corpus"), api("snapshot").catch(() => ({}))]);
      const reports = ((snap || {}).snapshot || {}).reports || {};
      // A work report is the performer's attributed claim, not a record: it
      // is shown as "reported", beside the rung the records establish.
      const reported = (alias) => {
        const r = reports[alias];
        if (!r) return "";
        if (r.error) return el("span", { class: "badge bad", text: "report unreadable", title: r.error });
        return el("span", { class: "badge", text: "reported " + ((r.report || {}).work_state || "?") });
      };
      main.append(el("h2", { text: "Every Warrant" }),
        table(["Warrant", "title", "rung", "unmet", "reported", "command"], (d.warrants || []).map((w) => [
          code(w.alias), w.title || "", rungBadge(w.rung), String((w.unmet || []).length), reported(w.alias), code(w.command)])));
    },
    async help(main) {
      const d = await api("help");
      main.append(el("h2", { text: "What next" }));
      main.append(table(["who", "act", "Warrant", "why", "command"], (d.next || []).map((a) => [
        el("span", { class: "badge " + (a.actor === "human" ? "warn" : ""), text: a.actor }), a.action, code(a.warrant), a.why, code(a.command)])));
      if (d.nothing) main.append(el("p", { class: "muted", text: d.nothing }));
      main.append(el("h2", { text: `war check — ${d.counts.error} error(s), ${d.counts.warn} warning(s)` }));
      main.append(table(["rule", "×", "remedy", "", "what it does"], (d.remedies || []).map((r) => [
        code(r.rule), String(r.count), r.command ? code(r.command) : el("span", { class: "muted", text: "none on record" }),
        r.act_id ? actButton(r.act_id, "Run") : el("span", { class: "muted", text: r.kind || "" }), r.purpose || ""])));
    },
  };

  // How a tick was earned (OW-WAR-0148 M13). Each level has its own word; a
  // claimed tick is drawn muted and never as a checked one, a tick below its
  // minimum as a warning. The page's stylesheet is unchanged (its bytes are
  // pinned by OW-WAR-0139's OBL-005), so the existing classes carry it.
  const TICK_CLASS = { claimed: "badge muted", observed: "badge ok", independent: "badge ok", signed: "badge ok" };
  function tickClass(level, meets) { return meets ? (TICK_CLASS[level] || "badge muted") : "badge warn"; }
  function tickBadge(i) {
    const t = i.tick;
    if (!t) return null;
    return el("span", { class: tickClass(t.level, t.meets_minimum), "data-tick": t.level,
      text: i.tick_marker || "(" + t.level + ")", title: t.unbacked ? "its [" + t.written + "] marker is not believed: " + t.unbacked : t.level });
  }
  function tickSummary(c) {
    if (!c) return "";
    const parts = [["claimed", c.claimed], ["observed", c.observed], ["independent", c.independent], ["signed", c.signed]]
      .filter(([, n]) => n > 0).map(([k, n]) => n + " " + k);
    return parts.join(", ") + (c.below_minimum ? " (" + c.below_minimum + " below minimum)" : "");
  }

  // Loopback only: claim or finish a ticket item. The server runs the same
  // ticket command the CLI would and answers with its words or its refusal.
  function ticketButton(act, target, label) {
    const b = el("button", { type: "button", text: label });
    b.addEventListener("click", async () => {
      const body = { act, target };
      if (act === "done") {
        const note = window.prompt("What you did (optional):", "");
        if (note === null) return;
        if (note) body.note = note;
      }
      b.disabled = true;
      try {
        const r = await fetch("/api/ticket", { method: "POST", headers: { Authorization: "Bearer " + token, "Content-Type": "application/json" }, body: JSON.stringify(body), cache: "no-store", credentials: "omit" });
        const text = await r.text();
        let d = {}; try { d = JSON.parse(text); } catch (_) { d = { message: text }; }
        showAct((r.ok ? "" : "refused" + (d.rule ? " (" + d.rule + ")" : "") + ": ") + (d.message || d.error || r.statusText));
        version = null;
      } catch (e) { showAct("refused: " + e.message); b.disabled = false; }
    });
    return b;
  }
  // A device's act carries a nonce the server issued to it, spent once.
  const actBody = async (id) => lan ? { id, nonce: (await api("nonce")).nonce } : { id };
  function actButton(id, label) {
    const b = el("button", { type: "button", text: label });
    b.addEventListener("click", async () => {
      b.disabled = true;
      try { await api("act", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(await actBody(id)) }); watchAct(); }
      catch (e) { showAct("refused: " + e.message); b.disabled = false; }
    });
    return b;
  }
  // LAN only: mark a signing act requested at the host. Nothing starts.
  function requestButton(id) {
    const b = el("button", { type: "button", text: "Ask at the host" });
    b.addEventListener("click", async () => {
      b.disabled = true;
      try { const r = await api("request", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(await actBody(id)) }); showAct("asked at the host: " + r.command); version = null; }
      catch (e) { showAct("refused: " + e.message); b.disabled = false; }
    });
    return b;
  }
  // LAN only: the pairing page. The code goes to the server in a POST body;
  // the human at the host confirms at its terminal.
  function pairPage() {
    const main = $("main");
    const go = el("button", { type: "button", text: "Ask the host to pair this device" });
    const out = el("p", { class: "muted" });
    go.addEventListener("click", async () => {
      go.disabled = true;
      out.textContent = "Waiting for the human at the host to answer at its terminal…";
      try {
        const r = await fetch("/pair", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ code: pairCode }), cache: "no-store", credentials: "same-origin" });
        if (!r.ok) throw new Error((await r.text()) || r.statusText);
        const d = await r.json();
        out.textContent = "Paired as device " + d.device + " until " + d.expires_at + ".";
        history.replaceState(null, "", location.pathname + "#p=tickets");
        page = "tickets"; poll();
      } catch (e) { out.textContent = "Not paired: " + (e.message || e); }
    });
    main.replaceChildren(el("div", {},
      el("h2", { text: "Pair this device" }),
      el("p", {}, "Before you continue, compare this certificate fingerprint with the one the host printed:"),
      el("p", {}, code(pairFp.replace(/(..)(?!$)/g, "$1:"))),
      el("p", { class: "note", text: "A paired device reads this program, runs automatic remedies and can ask for a signature at the host. It can never sign." }),
      el("p", {}, go), out));
  }
  function showAct(text, pre) {
    const f = $("act"); f.hidden = false; f.replaceChildren(el("div", { text }), pre ? el("pre", { text: pre }) : null);
  }
  async function watchAct() {
    const s = await api("act");
    if (s.state === "running") { showAct(`running: ${s.command} (${s.started_secs_ago}s) — answer your key's dialog`); setTimeout(watchAct, 1000); }
    else if (s.state === "done") { showAct(`${s.command} exited ${s.exit}`, s.output); version = null; }
  }

  async function render() {
    for (const a of document.querySelectorAll("nav a")) a.classList.toggle("active", a.dataset.page === page);
    const main = $("main");
    const fresh = el("div", {});
    try { await (pages[page] || pages.progress)(fresh); }
    catch (e) { fresh.replaceChildren(el("p", { class: "note", text: String(e.message || e) })); }
    main.replaceChildren(fresh);
  }
  async function poll() {
    try {
      const v = await api("version");
      $("program").textContent = v.program;
      $("status").textContent = "live · " + new Date().toLocaleTimeString() + " · war " + v.war;
      if (v.version !== version) { version = v.version; await render(); }
    } catch (e) { $("status").textContent = String(e.message || e); }
    setTimeout(poll, 2000);
  }
  window.addEventListener("hashchange", () => {
    const p = new URLSearchParams(location.hash.slice(1)).get("p");
    if (p && p !== page) { page = p; render(); }
  });
  if (lan && pairCode) pairPage(); else poll();
})();
