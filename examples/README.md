# Examples

Minimal, working IR files and their emitted targets. Run:

```
cargo build
# C# target (needs dotnet to run)
target/debug/irc to-cs examples/metric.ir > metric.cs          # C# idiom in,
                                                                # .NET out
target/debug/irc from-cs metric.cs                              # and back

# JS target (needs node to run)
target/debug/irc to-js examples/compact.ir > compact.js         # JS idiom in
                                                                # node out
```

## What you'll see

### `metric.ir` → `metric.cs`

C#-idiom rows (`char.IsLetter`, `input[..^1]`, `Math.Pow`) — canon keeps
the receiver-expressions verbatim and emits tight .NET. Char literals
(`'k'`) canonize as strings (the fidelity row 17; the `Char` type rides
signatures), so `IndexOf(last)` calls stay intact.

### `compact.ir` → `compact.js`

JS-idiom rows (`s.trim()`, `.split(" ").filter(...).map(...)`,
`.join(""), .slice(...)`) — canon drops the type ceremony and emits
runnable ESM.

### The canon file

Read the `.ir` rows: every row is a value. `var` is immutable,
`var mut` reassignable, `export fn` = the callable surface, `//` rows
exist only in hand canon (the machine printer drops them). The same canon
text lowers to both targets when the row is target-neutral; library
method spellings follow the target idiom (see `docs/tutorial.md` §6 for
the mapping table).

## Verifying

The full corpus gates live in `cargo test` (canon identity, round-trip,
behavior harness) and `bench/` (token bench, loop runs). These examples
smoke the same code paths.

## Limits (documented, honest)

- char literals canon to strings (fidelity row 17 — no Char atom yet)
- data-braces `{{…}}` in plain strings mark literal braces (C#-reader
  distinguishes; JS canon interpolates braces)
- `where` constraints, comments, formatting drop by contract
- the C# reader is 5.6 leg-1: data rows + fields restore; behavior needs
  the char-atom decision (see `docs/PLAN.md` Phase 6)