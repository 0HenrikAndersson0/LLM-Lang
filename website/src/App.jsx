const REPO = "https://github.com/0HenrikAndersson0/LLM-Lang";
const RAW = `${REPO}/blob/main`;

function Nav() {
  return (
    <nav className="nav">
      <a className="brand" href="#top">
        <span className="brand-mark">@1</span> LLM-Lang
      </a>
      <div className="nav-links">
        <a href="#proof">Proof</a>
        <a href="#how">How it works</a>
        <a href="#use">Use it</a>
        <a href="#agents">For agents</a>
        <a href="#honest">Honesty</a>
        <a href="#faq">FAQ</a>
      </div>
      <a className="nav-cta" href={REPO} target="_blank" rel="noreferrer">
        GitHub <span className="arrow">→</span>
      </a>
    </nav>
  );
}

function Hero() {
  return (
    <header className="hero" id="top">
      <p className="kicker not-chat">for agents, not for humans</p>
      <h1>
        Source code is <em>bloated gossip</em>.
        <br />
        Canon is what agents should read.
      </h1>
      <p className="hero-sub">
        LLM-Lang compiles typed + documented code into <strong>canon</strong> —
        a compact transfer representation — and emits it back out to JS/TS or
        C#, round-trip stable. Every executable byte survives. The chitchat
        doesn't.
      </p>
      <div className="stat-strip">
        <div className="stat">
          <div className="stat-num">57.3%</div>
          <div className="stat-label">tokens on fresh, never-seen code <a className="proof" href={`${RAW}/bench/results-local-projects.md`}>[proof]</a></div>
        </div>
        <div className="stat">
          <div className="stat-num">61,580</div>
          <div className="stat-label">/ 61,581 vendored tests pass, round-tripped <a className="proof" href={`${RAW}/bench/results-humanizer-rig.md`}>[proof]</a></div>
        </div>
        <div className="stat">
          <div className="stat-num">100%</div>
          <div className="stat-label">canon identity — a canon file is its own fixed point <a className="proof" href={`${RAW}/bench/results-round-trip.md`}>[proof]</a></div>
        </div>
      </div>
      <div className="hero-ctas">
        <a className="btn primary" href="#use">
          Try it <span className="arrow">→</span>
        </a>
        <a className="btn ghost" href="#how">
          How it works
        </a>
      </div>
    </header>
  );
}

const SRC_LINES = [
  ["doc", `/// <summary>Builds an amount from a decimal in major units.</summary>`],
  ["doc", `/// <param name="amount">Major-unit value, e.g. 12.50.</param>`],
  ["doc", `/// <param name="curr">Currency to denominate in.</param>`],
  ["doc", `/// <exception cref="ArgumentOutOfRangeException">If <paramref name="amount"/>`],
  ["doc", `/// has more decimals than the currency's scale.</exception>`],
  ["code", `public static Money Of(decimal amount, Currency curr)`],
  ["code", `{`],
  ["code", `    var scale = Scale[curr];`],
  ["code", `    var minor = decimal.Round(amount, scale) * (decimal)Math.Pow(10, scale);`],
  ["code", `    if (minor % 1m != 0m)`],
  ["code", `    {`],
  ["code", `        throw new ArgumentOutOfRangeException(nameof(amount), amount,`],
  ["code", `            "residual decimals after scaling");`],
  ["code", `    }`],
  ["code", `    return new Money((long)minor, curr);`],
  ["code", `}`],
];

const CANON_LINES = [
  ["code", `export fn Of(amount Decimal, curr Currency) Money{`],
  ["code", `  var scale=Scale[curr]`],
  ["code", `  var minor=decimal.Round(amount, scale)*(Decimal)Math.Pow(10, scale)`],
  ["code", `  if minor%1m!=0m{`],
  ["code", `    throw new ArgumentOutOfRangeException(nameof(amount), amount, "residual decimals after scaling")`],
  ["code", `  }`],
  ["code", `  return new Money((long)minor, curr)`],
  ["code", `}`],
];

const CodePane = {
  doc: (s) => <span className="l-doc">{s}{"\n"}</span>,
  code: (s) => <span>{s}{"\n"}</span>,
};

function SideBySide() {
  return (
    <section className="sbs">
      <p className="kicker">the whole trick, visible</p>
      <h2>Same function. Every semantic byte. Zero gossip.</h2>
      <p className="section-sub">
        These are real rows from the fresh-code gate — not a hand-made
        diagram. The XML docs are the five lines agents never needed. The
        code is the code.
      </p>
      <div className="sbs-grid">
        <div className="pane">
          <div className="pane-head">
            <span>typed + documented source</span>
            <span className="pane-meta">1032 tok / project (o200k)</span>
          </div>
          <pre className="code">
            {SRC_LINES.map(([t, s], i) => CodePane[t](s))}
          </pre>
        </div>
        <div className="sbs-arrow">
          <span>from-cs</span>
          <span className="arrow">→</span>
        </div>
        <div className="pane accent-pane">
          <div className="pane-head">
            <span>canon (.ir)</span>
            <span className="pane-meta">586 tok / project (o200k)</span>
          </div>
          <pre className="code">
            {CANON_LINES.map(([t, s], i) => CodePane[t](s))}
          </pre>
        </div>
      </div>
      <p className="footnote">
        Both panes: real rows from <code>bench/corpus/local/</code>. The canon
        file re-emits the exact function, compiles under <code>dotnet build</code>,
        and runs correct behavior.
      </p>
    </section>
  );
}

function Proof() {
  const gates = [
    ["Fresh-code gate — code the language never saw", "57.3% o200k / 57.9% llama of typed+documented sources; emitted C# compiles + runs correct behavior; emitted JS passes behavior harness", "bench/results-local-projects.md"],
    ["Humanizer behavior rig — vendored 61,581-test suite run against round-tripped code", "61,580 pass · 0 fail · 1 vendor-disabled skip — inside the real library tree", "bench/results-humanizer-rig.md"],
    ["Whole-corpus machine canon", "JS 36.7% o200k · C# 64.2% o200k · both weighted 58.3%", "bench/results-6p1.md"],
    ["Round-trip (JS leg)", "double round-trip compiles + behaves identically (node 7/7, tsc 7/7, behavior 3/3)", "bench/results-round-trip.md"],
    ["Canon identity", "35/35 files are their own fixed point (ircc verify exits 1 on drift)", "crates/cli/src/main.rs"],
  ];
  return (
    <section className="proof" id="proof">
      <p className="kicker">proof, not vibes</p>
      <h2>We measured everything. The receipts are in the repo.</h2>
      <p className="section-sub">
        Every claim here links to the actual benchmark report — the numbers
        are the ones the toolchain itself produced, gate by gate.
      </p>
      <div className="gate-table">
        {gates.map(([g, r, p]) => (
          <div className="gate-row" key={g}>
            <div className="gate-name">{g}</div>
            <div className="gate-result">{r}</div>
            <a className="proof" href={`${RAW}/${p}`}>[proof]</a>
          </div>
        ))}
      </div>
      <div className="big-nums">
        <div>
          <div className="big-num">61,580</div>
          <div className="big-label">
            of 61,581 vendored tests pass against round-tripped code{" "}
            <a className="proof" href={`${RAW}/bench/results-humanizer-rig.md`}>[proof]</a> — the strongest gate in the repo
          </div>
        </div>
        <div>
          <div className="big-num">8</div>
          <div className="big-label">emit rows the fresh-code gate caught — and fixed — in one pass</div>
        </div>
        <div>
          <div className="big-num">+2</div>
          <div className="big-label">tokens: the honest corpus-wide cost of carrying where-constraints, get-only props and ulong semantics</div>
        </div>
      </div>
    </section>
  );
}

function How() {
  const steps = [
    ["1", "Compile in", "ircc from-js app.ts > app.ir — or from-cs. Comments, XML docs and doc-bloat drop; signatures, types and every executable line survive as canon rows."],
    ["2", "Verify", "ircc verify app.ir — canon identity as a lint. A canon file must be its own fixed point: file == print(expand(file)). Exit 1 on drift, every time."],
    ["3", "Agents work in canon", "Read, edit, review, diff in the compact surface. It is designed for exactly the loop agents already do — the repair harness proved it on qwen-class models and up."],
    ["4", "Emit back out", "ircc to-js app.ir > app.js (with --map for line-mapped feedback) or to-cs. The output compiles and behaves like the original — proven, not assumed."],
  ];
  return (
    <section className="how" id="how">
      <p className="kicker">the pipeline</p>
      <h2>Four verbs. That's the whole toolchain.</h2>
      <div className="steps">
        {steps.map(([n, t, d]) => (
          <div className="step" key={n}>
            <div className="step-num">{n}</div>
            <div>
              <h3>{t}</h3>
              <p>{d}</p>
            </div>
          </div>
        ))}
      </div>
      <div className="pipe-note">
        <code>Source (JS/TS, C#)</code> →{" "}
        <code>ircc from-*</code> → <code className="hl">canon .ir</code> →{" "}
        <code>ircc to-*</code> → <code>Target (JS, C#)</code>
      </div>
    </section>
  );
}

function Use() {
  return (
    <section className="use" id="use">
      <p className="kicker">use it</p>
      <h2>One binary. Five verbs. No config.</h2>
      <pre className="code install-block">
        <span className="l-cmt"># install</span>{"\n"}
        <span className="l-kw">cargo install llm-lang</span>{"\n\n"}
        <span className="l-cmt"># read + write canon</span>{"\n"}
        <span className="l-kw">ircc from-js  service.ts  {'>'} service.ir</span>{"\n"}
        <span className="l-kw">ircc to-js    service.ir  {'>'} service.js</span>{"\n"}
        <span className="l-kw">ircc to-cs    service.ir  {'>'} service.cs</span>{"\n\n"}
        <span className="l-cmt"># the lint that keeps canon honest — run it before proposing changes</span>{"\n"}
        <span className="l-kw">ircc verify   *.ir</span>{"\n\n"}
        <span className="l-cmt"># the language ships its own docs, embedded</span>{"\n"}
        <span className="l-kw">ircc doc spec | tutorial | fidelity | versioning</span>
      </pre>
      <p className="section-sub">
        Currently pre-release (v0.1-rc, crates.io publish pending). Building
        from source is one <code>cargo build --release</code> away —{" "}
        <a href={REPO}>repo instructions</a>.
      </p>
    </section>
  );
}

function Agents() {
  return (
    <section className="agents" id="agents">
      <p className="kicker">for agent builders</p>
      <h2>The language distributes itself to your agent.</h2>
      <p className="section-sub">
        Canon was designed to be authored and repaired by models — so the
        binary ships the training material. One command installs the skill
        into the tool conventions agents already speak.
      </p>
      <pre className="code install-block">
        <span className="l-kw">ircc install-skill</span>          <span className="l-cmt"># → ~/.agents/skills (neutral)</span>{"\n"}
        <span className="l-kw">ircc install-skill --claude</span>   <span className="l-cmt"># → ~/.claude/skills (Claude Code)</span>{"\n"}
        <span className="l-kw">ircc install-skill --opencode</span> <span className="l-cmt"># → ./.opencode/skills (opencode)</span>{"\n"}
        <span className="l-kw">ircc install-skill --agents-md</span> <span className="l-cmt"># → ./AGENTS.md (Codex, Gemini CLI, Cursor)</span>
      </pre>
      <p className="section-sub">
        The skill carries the command surface, the four canon-identity rules,
        and a row-shape cheat sheet. The full grammar prints from{" "}
        <code>ircc doc spec</code> — embedded in the binary, no repo needed.
      </p>
    </section>
  );
}

function Honest() {
  return (
    <section className="honest" id="honest">
      <p className="kicker">the honesty section</p>
      <h2>Where the savings stop — measured, not hand-waved.</h2>
      <div className="honest-grid">
        <div className="honest-card">
          <h3>Savings are conditional on the baseline.</h3>
          <p>
            Against typed + documented sources: <strong>~57%</strong>. Against
            bare doc-free JS runtime: canon is <em>~110%</em> — it carries
            signatures and type structure that bare JS throws away. The claim
            class is documented code, and we say so on the tin.
          </p>
        </div>
        <div className="honest-card">
          <h3>Comments don't survive. On purpose.</h3>
          <p>
            Drops are by contract, per-row in the fidelity budget. Things with
            no core form capture verbatim as <code>opaque</code> rows and
            re-emit byte-identically — enums, operators, <code>#if</code>{" "}
            regions.
          </p>
        </div>
        <div className="honest-card">
          <h3>We broke it before shipping it.</h3>
          <p>
            The fresh-code gate found 8 real emit bugs the rehearsed corpus
            never exercised — switch arms, enum emission, dict initializers,
            destructured lambdas. All fixed, gates green, suite re-run at zero
            failures. That's why you gate on unseen code.
          </p>
        </div>
      </div>
    </section>
  );
}

const FAQ = [
  ["Is it lossless?", "Semantically: yes — behavior identity is gated (vendor suite 61,580/61,581, 1 skip by the vendor's own config). Textually: comments, XML docs and formatting drop by contract, and the drops are itemized in the fidelity budget. Constructs without core forms capture as opaque rows and come back byte-identical."],
  ["Why not just minify, or strip comments with regex?", "Minifiers for JS strip whitespace; canon is a full transfer representation for BOTH targets: types, signatures, modifiers, generics, control flow — each with a core form, fixed-point identity as a lint, and a repair loop that feeds emit errors back to the right canon row. Regex comment-stripping can't round-trip a C# record struct."],
  ["What does 'canon must be its own fixed point' mean?", "A canon file re-parses to itself: expand(print(expand(file))) == file. ircc verify enforces it, so a drifting canon can never churn silently between tool rounds — the lint catches data loss that round-to-round comparisons hide."],
  ["What did you actually verify the behavior with?", "Humanizer's own 61,581-test suite, run against the round-tripped corpus compiled inside the real vendored library tree: 61,580 pass, 0 fail, 1 vendor-disabled performance gate. Plus behavior harnesses on the JS side (3/3 subjects) and a fresh-code gate on locally authored projects."],
  ["Does my agent need to be a big model?", "The repair loop ran on local qwen-class models (7B) for the original go/no-go. Canon's job is to make the input small and the feedback precise — that helps small models most, but nothing here needs frontier scale."],
  ["What about plain Python, Go, Rust?", "Canon v1.1 has two targets: JS/TS and C#. The core-form design is additive-only, so more targets land as new rows over targets, not rewrites. Versioning policy is embedded: ircc doc versioning."],
];

function Faq() {
  return (
    <section className="faq" id="faq">
      <p className="kicker">FAQ</p>
      <h2>We give an FAQ.</h2>
      <div className="faq-list">
        {FAQ.map(([q, a]) => (
          <details key={q} className="faq-item">
            <summary>{q}</summary>
            <p>{a}</p>
          </details>
        ))}
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="footer">
      <div className="footer-top">
        <div>
          <div className="brand-mark">@1</div>
          <p>
            canon: the representation agents read between tool rounds —
            verified, measured, round-trip stable.
          </p>
        </div>
        <div className="footer-links">
          <a href={REPO}>GitHub</a>
          <a href={`${RAW}/bench/results-local-projects.md`}>Fresh-code gate</a>
          <a href={`${RAW}/bench/results-humanizer-rig.md`}>Humanizer rig</a>
          <a href={`${RAW}/crates/cli/assets/docs/spec.md`}>Canon spec</a>
        </div>
      </div>
      <div className="footer-meta">
        <span>v0.1-rc — crates.io publish pending</span>
        <span>MIT corpus sources (Humanizer, cheerio) — license notes in-repo</span>
        <span>canon v1.1 · @1</span>
      </div>
    </footer>
  );
}

export default function App() {
  return (
    <>
      <Nav />
      <Hero />
      <SideBySide />
      <Proof />
      <How />
      <Use />
      <Agents />
      <Honest />
      <Faq />
      <Footer />
    </>
  );
}