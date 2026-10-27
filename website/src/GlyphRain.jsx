import { useEffect, useRef } from "react";

const GLYPHS = "@1 @ns @use type fn export ctor var mut if for while switch try catch finally return opaque get where sealed record struct interface => -> { } ( ) [ ] < > :: .. ..= => ?".split(" ");

const COLUMN_GLYPHS = [
  "@ns Ledger.Core", "type Money export record{", "export fn TryAdd(a Money) Bool{",
  "var mut Balance = 10.50m", "if minor%1m!=0m{", "}else{", "return [.. items . Where(p => p.Active)]",
  "mode switch{ Floor => 0, _ => 1 }", "try risky() catch NotFound return null",
  "export get String Label { get; }", "type Cache<T> : Object where T : class{",
  "fn Of(amount Decimal, curr Currency) Money{", "@1 @use System.Runtime.CompilerServices",
  "opaque:public enum Currency {", "export ctor (id Long, total Decimal)",
];

export default function GlyphRain() {
  const ref = useRef(null);
  useEffect(() => {
    const canvas = ref.current;
    const ctx = canvas.getContext("2d");
    let raf;
    let w, h, dpr;
    let cols = [];

    const resize = () => {
      dpr = Math.min(2, window.devicePixelRatio || 1);
      w = canvas.clientWidth;
      h = canvas.clientHeight;
      canvas.width = w * dpr;
      canvas.height = h * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      cols = Array.from({ length: Math.max(7, Math.floor(w / 210)) }, (_, i) => ({
        x: (i / Math.max(1, Math.floor(w / 210))) * w + Math.random() * 90,
        y: Math.random() * h - h,
        speed: 0.22 + Math.random() * 0.5,
        glyph: COLUMN_GLYPHS[Math.floor(Math.random() * COLUMN_GLYPHS.length)],
        alpha: 0.05 + Math.random() * 0.09,
        vy: 0.05 + Math.random() * 0.12,
      }));
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(canvas);

    const drift = [
      { x: 0, y: 0, vx: 0.045, vy: 0.02, a: 0.028, s: GLYPHS[0], dx: 0, dy: 0, ox: 0, oy: 0 },
    ];
    const bits = Array.from({ length: 46 }, () => ({
      x: Math.random(),
      y: Math.random(),
      vx: (Math.random() - 0.5) * 0.00028,
      vy: (Math.random() - 0.5) * 0.00021,
      g: GLYPHS[Math.floor(Math.random() * GLYPHS.length)],
      a: 0.03 + Math.random() * 0.07,
    }));

    let t = 0;
    const draw = () => {
      t += 1;
      ctx.clearRect(0, 0, w, h);
      ctx.font = "12px SFMono-Regular, ui-monospace, monospace";

      for (const b of bits) {
        b.x = (b.x + b.vx + 1) % 1;
        b.y = (b.y + b.vy + 1) % 1;
        const glowA = 0.5 + 0.5 * Math.sin(t * 0.01 + b.x * 40);
        ctx.fillStyle = `rgba(139, 92, 246, ${(b.a * glowA).toFixed(3)})`;
        ctx.fillText(b.g, b.x * w, b.y * h);
      }

      for (const c of cols) {
        c.y += c.vy;
        if (c.y > h + 40) {
          c.y = -60;
          c.x = Math.random() * w;
          c.glyph = COLUMN_GLYPHS[Math.floor(Math.random() * COLUMN_GLYPHS.length)];
        }
        const grad = ctx.createLinearGradient(c.x, c.y, c.x, c.y + 22);
        grad.addColorStop(0, `rgba(139, 92, 246, ${c.alpha})`);
        grad.addColorStop(1, `rgba(34, 211, 238, ${c.alpha * 0.5})`);
        ctx.fillStyle = grad;
        ctx.fillText(c.glyph, c.x, c.y);
      }
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  }, []);
  return <canvas ref={ref} className="glyph-rain" aria-hidden="true" />;
}