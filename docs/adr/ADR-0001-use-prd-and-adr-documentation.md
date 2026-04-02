# ADR-0001: Use PRD and ADR Documentation for Project Governance

**Date:** 2026-04-02

## Status

Accepted

## Context

The openEHR Explorer project is a new cross-platform desktop application. As the project grows, we need a lightweight but structured way to:

1. Capture product requirements so that contributors understand what we're building and why.
2. Record significant architectural and technical decisions so that future contributors (and our future selves) understand why the codebase looks the way it does.

Without a documentation standard, decisions get buried in chat logs, PR descriptions, or individual memory. This makes onboarding harder and leads to relitigating settled decisions.

## Decision

We will use two complementary documentation formats stored in the repository:

**Product Requirements Documents (PRDs)** stored under `docs/prd/` with the naming convention `PRD-XXXX-short-title.md`. PRDs capture the what and why of features — problem statements, user stories, functional requirements, success criteria, and implementation milestones.

**Architecture Decision Records (ADRs)** stored under `docs/adr/` with the naming convention `ADR-XXXX-short-title.md`. ADRs follow Michael Nygard's template (Status, Context, Decision, Consequences) and capture the why behind significant technical choices — framework selection, data storage strategy, API design patterns, etc.

Both use sequential four-digit numbering starting from 0001.

## Consequences

- Every significant product decision will have a corresponding PRD that can be referenced in issues and PRs.
- Every significant architectural decision will have a corresponding ADR that explains the rationale and trade-offs.
- New contributors can read the `docs/` directory to quickly understand both what the project does and why it's built the way it is.
- There is a small overhead to writing these documents, but the cost is far outweighed by the clarity they provide, especially for a project that aims to attract open-source contributors.
- PRDs and ADRs are living documents — they can be updated as decisions evolve, with status changes (e.g., "Superseded by ADR-XXXX") to maintain a clear history.
