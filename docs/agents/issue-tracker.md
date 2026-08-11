# Issue tracker: Local Markdown

## Agent metadata

- **Purpose:** locate and update repository-local issue/spec records.
- **Read when:** the user references a ticket/spec or a skill asks to publish or
  fetch an issue.
- **Authority:** `.scratch/<feature>/` is the issue store; GitHub/Jira mutation
  is out of scope unless explicitly requested and available.

Issues and specs for this repo live as Markdown files in `.scratch/`.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`
- The spec is `.scratch/<feature-slug>/spec.md`
- Implementation issues are one file per ticket at `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01`—never a single combined tickets file
- Comments and conversation history append to the bottom of the file under a `## Comments` heading

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<feature-slug>/`, creating the directory if needed.

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or issue number directly.

## Wayfinding operations

The map is a file with one child file per ticket.

- **Map:** `.scratch/<effort>/map.md`—the Notes, Decisions-so-far, and Fog body
- **Child ticket:** `.scratch/<effort>/issues/NN-<slug>.md`, numbered from `01`, with the question in the body. A `Type:` line records the ticket type (`research`, `prototype`, `grilling`, or `task`); a `Status:` line records `claimed` or `resolved`.
- **Blocking:** A `Blocked by: NN, NN` line near the top. A ticket is unblocked when every listed file is `resolved`.
- **Frontier:** Scan `.scratch/<effort>/issues/` for files that are open, unblocked, and unclaimed; first by number wins.
- **Claim:** Set `Status: claimed` and save before any work.
- **Resolve:** Append the answer under an `## Answer` heading, set `Status: resolved`, then append a context pointer—gist plus link—to the map’s Decisions-so-far section.
