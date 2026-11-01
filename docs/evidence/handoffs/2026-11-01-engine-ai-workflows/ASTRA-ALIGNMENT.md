# Astra skill-alignment audit

Read-only review of the current repo-local `engine-architecture` and `engine-module-design` skills and `notes/engine-rebuild/AI-DEVELOPMENT.md`, compared with [OpenAI's Astra guidance](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra) and the local `skill-creator` instructions. This is an instruction audit, not a model or workflow evaluation.

## One targeted edit

**Make the game-creation loop explicitly illustrative.** `AI-DEVELOPMENT.md:152–155` says an agent creating a game adds a “seed action/count/gate rule” and then tests “actual pointer input, focus, picking and feedback.” As written, it could turn one useful I3 fixture into a compulsory itinerary for headless, keyboard, controller or other game projects. OpenAI advises concise, discriminating skills and completion boundaries without overly specific recipes. Label this as an *example pointer-driven smoke test*, or say to exercise the actual input path chosen by the game. Keep the important invariant: a headless semantic-action fixture does not prove the rendered input path, and evidence must identify the exact changed build.

## No skill-entrypoint change recommended

- `engine-architecture` has a narrow whole-engine description; `engine-module-design` targets new or substantially redesigned modules (`SKILL.md:3` in each). The module skill explicitly excludes ordinary bug fixes from engine redesign (`engine-module-design/SKILL.md:13–16`) and reads only task-relevant references (`:28–37`). These fit progressive disclosure.
- Their analytical derivation, shared state ownership, producer/consumer coordination, proof status and incomplete-mechanism distinctions are project-specific decision rules, not generic handholding. The architecture skill's design-programme link is conditional on the coordinated research programme (`engine-architecture/SKILL.md:83–88`), and the module skill routes contract changes back to its owner (`engine-module-design/SKILL.md:71–73`). Retain these.
- `AI-DEVELOPMENT.md:38–41, 61–85` already describes short scoped instructions, a small skill set, progressive references and no skill-created permission system. Its proposed operations are identified as requirements rather than implemented commands (`:122–143`); its behavioral evaluation is future work (`:205–211`). Do not claim Astra behavior has been tested from this review.
