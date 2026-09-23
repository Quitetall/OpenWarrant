// war ui — the page. Every value from the server is written with
// textContent, never parsed as HTML, so a record cannot inject markup. The
// session token arrives in the URL fragment, moves into this closure, and
// the fragment is cleared: it is never in a cookie, storage or a request
// URL. Acts send a row id; the server decides what runs.
"use strict";
(() => {
  const params = new URLSearchParams(location.hash.slice(1));
  const token = params.get("t") || "";
  let page = params.get("p") || "progress";
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
  const api = async (path, init) => {
    const r = await fetch("/api/" + path, {
      ...init,
      headers: { Authorization: "Bearer " + token, ...(init && init.headers) },
      cache: "no-store",
      credentials: "omit",
    });
    if (r.status === 401) throw new Error("session token missing or wrong — open the link `war ui` printed");
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
    },
    async queue(main) {
      const d = await api("queue");
      main.append(el("h2", { text: "Awaiting a signature" }),
        el("p", { class: "note", text: "A button runs the same `war sign <target> --ssh-sign` a terminal would. Your key's confirm dialog is the signature — load the key with `ssh-add -c`, or a click signs without asking." }));
      if (d.who) main.append(el("p", { class: "note" }, d.who.why, " — restart with ", code(d.who.command)));
      if (d.signer) main.append(el("p", { class: "muted" }, "signing as ", code(d.signer)));
      if (!(d.acts || []).length) main.append(el("p", { class: "muted", text: "Nothing awaits a signature." }));
      else main.append(table(["#", "act", "dry run", "", "command"], d.acts.map((a) => [
        String(a.n), el("span", {}, a.act, " ", code(a.target)),
        el("span", { class: "badge " + (a.verdict === "would record" ? "ok" : a.verdict === "needs a decision" ? "warn" : "bad"), text: a.verdict, title: (a.reasons || []).join("\n") }),
        a.act_id ? actButton(a.act_id, "Sign")
          : (a.choices || []).length ? el("span", {}, ...a.choices.map((c) => actButton(c.act_id, c.label)))
          : el("span", { class: "muted", text: (a.reasons || [])[0] || "" }),
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

  function actButton(id, label) {
    const b = el("button", { type: "button", text: label });
    b.addEventListener("click", async () => {
      b.disabled = true;
      try { await api("act", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ id }) }); watchAct(); }
      catch (e) { showAct("refused: " + e.message); b.disabled = false; }
    });
    return b;
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
  poll();
})();
