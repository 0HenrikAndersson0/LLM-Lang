import { useState, useEffect } from "react";
import { useInView, useCountUp } from "./anim.js";

const LINES = [
  { cmd: "ircc from-js service.ts > service.ir", out: "  ok  29 rows canonized — docs dropped, semantics carried" },
  { cmd: "ircc verify service.ir", out: "  ok  fixed point: file == print(expand(file))" },
  { cmd: "ircc to-js --map service.ir > service.js", out: "  ok  node --check green — behavior identical" },
];

export default function Terminal() {
  const [ref, inView] = useInView({ threshold: 0.3 });
  const [step, setStep] = useState(0);
  const [typed, setTyped] = useState("");

  useEffect(() => {
    if (!inView || step >= LINES.length) return;
    const cmd = LINES[step].cmd;
    let i = 0;
    setTyped("");
    const iv = setInterval(() => {
      i += 1;
      setTyped(cmd.slice(0, i));
      if (i >= cmd.length) {
        clearInterval(iv);
        setTimeout(() => setStep((s) => s + 1), 520);
      }
    }, 34);
    return () => clearInterval(iv);
  }, [inView, step]);

  return (
    <div className="term" ref={ref}>
      <div className="term-head">
        <span className="dot red" /><span className="dot amber" /><span className="dot green" />
        <span className="term-title">agent — zsh</span>
      </div>
      <div className="term-body">
        {LINES.slice(0, step).map((l, i) => (
          <div key={i}>
            <div className="term-line"><span className="prompt">$ </span>{l.cmd}</div>
            <div className="term-out">{l.out}</div>
          </div>
        ))}
        {step < LINES.length && (
          <div className="term-line">
            <span className="prompt">$ </span>
            {typed}
            <span className="caret" />
          </div>
        )}
        {step >= LINES.length && (
          <div className="term-line"><span className="prompt">$ </span><span className="caret" /></div>
        )}
      </div>
    </div>
  );
}

export function TokenBars() {
  const [ref, inView] = useInView();
  const bars = [
    { label: "typed + documented source", tokens: 3913, pct: 100, cls: "bar-src" },
    { label: "canon .ir", tokens: 2241, pct: 57.3, cls: "bar-canon" },
    { label: "bare doc-free JS (the floor)", tokens: 2025, pct: 51.7, cls: "bar-bare" },
  ];
  return (
    <div className="bars" ref={ref}>
      <div className="bars-title">tokens per project set · o200k · the fresh-code gate</div>
      {bars.map((b, i) => (
        <Bar key={b.label} b={b} inView={inView} delay={i * 220} />
      ))}
      <div className="bars-note">
        canon = 57.3% of the source it compiles from — and 110% of the doc-free
        floor. Signed: the honesty section.
      </div>
    </div>
  );
}

function Bar({ b, inView, delay }) {
  const w = useCountUp(b.pct, inView, 1300, (v) => v);
  const n = useCountUp(b.tokens, inView, 1500, (v) => v.toLocaleString("en-US", { maximumFractionDigits: 0 }));
  return (
    <div className="bar-row">
      <div className="bar-label">{b.label}</div>
      <div className="bar-track">
        <div
          className={`bar ${b.cls}`}
          style={{
            width: inView ? `${w}%` : "0%",
            transitionDelay: `${delay}ms`,
          }}
        />
      </div>
      <div className="bar-tokens">{n}</div>
    </div>
  );
}