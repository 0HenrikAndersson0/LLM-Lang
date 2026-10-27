# GEMINI.md

Read and follow `AGENTS.md` at the repository root — it is the canonical
agent workflow for this repo (LLM-Lang canon: reading, authoring, emit,
verify). Quick anchors:

- Canon v1.1 grammar reference: `crates/cli/assets/docs/spec.md`
- Authoring walkthrough (JS + C# idiom tables): `crates/cli/assets/docs/tutorial.md`
- Canon identity lint (run before proposing any change): `ircc verify <files>`

Canon files must be their own fixed point; drift breaks the pipeline.