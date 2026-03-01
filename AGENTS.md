
# AGENT.md

## Project Overview
Blockchain based app for desktop, android or iphone
Stack: Rust, Dioxus, Sqlite, WebRTC

## Architecture
- Clean architecture
- Services contain business logic

## Code Style
 - Write code and comments in english
 - for UP components, read DIOXUS_AGENTS.md
 - Avoid using Rr and Arc if possible. Use direct instance instead.
 - Use helper::Guid for GUID-Ids. (Found in helper = { git = "https://bitbucket.org/revwork/rust_helper.git", package = "rust_helper", features=["uuid", "sqlite"]  crates)
 - Keep low-complexity types in a single file (type, logic, DB access, UI). When a file grows or responsibilities expand, split into a module folder with submodules.

## Collaboration Mode
- For complex tasks: explain plan first
- If unsure: ask clarifying questions
- Prefer incremental changes
- Report only delta changes (new edits made in the current task), not the full working tree diff.
- Before applying a patch, verify whether the target lines are already present to avoid duplicate/redundant edits.
- In summaries, separate clearly: "already existing changes" vs "changes made in this task".
- When showing file changes, prefer file-local diffs for touched files instead of broad repository-level summaries.


If there is a conflict:
Project architecture > API stability > Framework conventions.

##

#Rust

# AGENTS_OWN.md

## Purpose of this File

This file stores project-specific insights, conventions, architectural decisions, and important technical knowledge.

It ensures that AI agents (e.g., Copilot or other coding assistants) can work in a context-aware, consistent, and high-quality manner.

---

## Instructions for Agents

1. **Always read this file first**
   Before making changes to the project, read `AGENTS_OWN.md` completely.

2. **If the file does not exist**
   - Analyze the current project structure.
   - Identify:
     - Programming languages used
     - Frameworks and libraries
     - Build and deployment mechanisms
     - Project architecture
     - Naming conventions
     - Testing strategy
     - Configuration patterns
   - Create this file and add a structured project summary.

3. **Maintain and Update**
   - Whenever significant architectural, structural, or workflow changes occur, update this file.
   - Add notable learnings, constraints, or recurring pitfalls.
   - Keep entries concise and factual.
   - Avoid duplicating information already well documented elsewhere.

---

## What to Document Here

- Architectural patterns
- Design decisions and their rationale
- Non-obvious dependencies
- Code conventions not enforced automatically
- Performance considerations
- Security constraints
- Environment-specific behavior
- Known limitations or technical debt
- Recurring implementation patterns
- Integration details with external systems

---

## Writing Style Guidelines

- Be concise and structured.
- Use bullet points where appropriate.
- Avoid long prose.
- Prefer actionable knowledge over generic explanations.
- Focus on information that improves future code generation quality.
