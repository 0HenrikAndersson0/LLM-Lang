# LLM-Lang, explained simply

*(no programming-language background assumed; for the technical version see
`README.md` and `docs/language-design.md`)*

## The problem

AI models are billed by the *token* — roughly, one per word or word-piece of
text. When an AI agent (a program that writes and edits code with an AI
model inside the loop) works with code, every line it writes or re-reads
costs tokens. Real project files are full of ceremony: punctuation, type
annotations, comments, repeated keywords. That ceremony costs money every
single time the model writes, reads, or re-reads the file.

So: what if we gave the agent a *condensed* way to write code — one that
strips the ceremony without losing the meaning — and translate it to
normal code (and back) automatically?

## What we built

A small, deliberately-designed shorthand ("IR" — intermediate representation)
and a command-line translator called `ircc`:

```
   agent writes this (short)          tool turns it into JavaScript
   ──────────────────────────         ──────────────────────────────
   fn camelCase(String str) String{   export function camelCase(str) {
     str.replace(/[._-](\w|$)/g, →      return str.replace(/[._-](\w|$)/g,
       (_, x)=>x.toUpperCase())            (_, x) => x.toUpperCase());
   }                                  }
```

*(real example from the test corpus — the shorthand omits the braces,
`return`, `function`, and boilerplate; the meaning is identical)*

- **Writing direction:** the shorthand goes in, JavaScript comes out
  (JavaScript first; C# is the next target).
- **Reading direction:** existing JavaScript/TypeScript files are converted
  *back* into the shorthand, so an agent can absorb an existing codebase in
  condensed form, or rewrite it in condensed form to work on it cheaply.
- The translator keeps the shorthand in one exact canonical shape, so the
  same code always costs the same, and the model can learn it once.

## What we proved with measurements

We scored the shorthand against real-world source files (from two MIT-
licensed open-source projects, Humanizer and cheerio) using the exact
token counters that the big models use:

1. **It saves tokens.** Hand-rendered shorthand for 16 real files cost
   **46%** of the original files' tokens — well past the 30% goal that
   justified the project.
2. **It round-trips faithfully.** Existing JS/TS files → shorthand → back
   to JS: the result compiles cleanly (7/7 files on both validators) and
   **behaves exactly like the original** on every automated test we could
   run — every single case matched.
3. **Agents can actually write it.** We ran the full loop with small local
   AI models (a 7-billion-parameter model on a laptop): they wrote features
   in the shorthand, got errors back from the running program, and fixed
   them. All target tasks eventually passed, some on the first try.
4. **The savings have a boundary — and we measured it honestly.** The 46%
   saving holds versus *typical* project files (which carry type
   annotations and documentation). Against a bare, comment-free, type-free
   JavaScript file, the shorthand can be *larger*. And when the agent
   keeps re-reading its own output during a task, that back-and-forth
   traffic partly eats the saving at small feature sizes. The practical
   takeaway: use the shorthand where files are read *many* times and are
   written *once* — e.g. spec/module files an agent keeps opening — which
   is exactly the "agent file" use-case the project targets.

## Honest limitations

- Comments and formatting are removed (that's where most of the saving
  comes from — and they're not needed by the machine).
- It's a lossy store: exotic type machinery and rare language corners are
  preserved as verbatim captured text rather than translated (every such
  capture is listed in our fidelity budget; none appeared in normal code
  beyond one type-heavy file).
- The loop tests used small local models; proving the same with frontier
  models is future work.

## Where we are in the plan

| stage | status |
|---|---|
| Design measured & frozen (grammar from real data) | done |
| Translator core: read shorthand → expand → canonical text | done |
| Shorthand → JavaScript | done, verified |
| JavaScript → shorthand (round-trip) | done, verified |
| C# target (same both-directions treatment) | next |
| Hardening, docs, first release | last |

Everything is evidenced in `docs/PLAN.md` (the engineering plan, with a
status per step) and the measurement reports under `bench/`.