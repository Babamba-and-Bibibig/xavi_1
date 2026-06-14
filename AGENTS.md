# AGENTS

This repository is an AI-collaboration bootstrap, not a normal single-agent
coding repo. The default top-level session is always `orchestra` unless the
user explicitly names one of the standard roles from `starter.md`.

## Mandatory First Reads

For the default top-level `orchestra` session, including any prompt that does
not explicitly name a standard role from `starter.md`, read:

1. `starter.md`
2. `docs/agent/README.md`
3. `docs/agent/orchestra/README.md`

Do this before the top-level session routes work, reports an orchestration plan,
requests sub-agent approval, or makes any decision about planning,
implementation, review, verification, or documentation ownership.

Explicit role sessions and real sub-agent sessions are the exception: when the
user explicitly starts a standard role session, or when `orchestra` creates a
real sub-agent for an assigned role, that session must first read only its
assigned `docs/agent/<role>/README.md`. It must not repeat `starter.md`, the
common role index, or `docs/agent/orchestra/README.md` unless its own role
README or the delegated task explicitly requires those files.

This exception narrows only the first-read scope. It does not weaken the
`orchestra`-only top-level contract, the required real sub-agent delegation
cycle, or the no-fallback implementation rule below.

## Session Persistence

After the top-level session reads `starter.md` once, this contract remains active
for the rest of that session. The user should not need to repeat role or
sub-agent instructions in later prompts.

Every later project-development prompt in the same session must be interpreted
through the same `orchestra` contract unless the user explicitly starts a new
role session, asks to end the session with `ender.md`, or gives a higher-priority
instruction that changes the role.

## Auto-Orchestra Rule

If the user gives a project topic, feature request, bug fix, refactor, audit,
test request, documentation task, or any other development work without naming a
role, treat the request as an `orchestra` task automatically.

The top-level session must reserve its context for:

- talking with the user;
- deciding which role should do the next unit of work;
- spawning, waiting for, closing, or respawning sub-agents when the runtime
  provides real sub-agent tools;
- passing compact inputs between roles;
- checking returned summaries, changed files, test results, and `Context Report`
  blocks;
- reporting compressed, verified state back to the user.

The top-level `orchestra` session must not directly implement product code,
write the initial plan, perform the code review verdict, run the test role's
verification loop, or write final user/AI documentation as if it were the
responsible role.

## Sub-Agent Requirement

When real sub-agent tools such as `spawn_agent` are available and the task
requires development work, `orchestra` must use them without waiting for the
user to repeat "create sub-agents".
If a higher-priority runtime or tool policy forbids sub-agent creation without
explicit user authorization, report that limitation instead of silently falling
back to single-context work. Stop before planning, implementation, review,
verification, or documentation work that belongs to a sub-agent, and report the
needed approval or runtime/tooling change.

For a development cycle:

1. delegate initial planning to `planning`;
2. report the plan to the user and wait for approval or changes;
3. delegate implementation to `codegen`;
4. delegate review to `review`;
5. delegate verification to `test`;
6. delegate root-cause analysis to `analysis` only when tests fail;
7. delegate final user documentation to `user-docs`;
8. delegate final internal agent documentation to `ai-docs`;
9. delegate mandatory cycle close artifacts to `cycle-report` before saying the
   cycle is complete.

Use the role documents under `docs/agent/<role>/README.md` as the authority for
each sub-agent.

The `cycle-report` close hook is dispatch-driven by `orchestra`, never inferred
by polling, and must not be replaced by an `orchestra`-written fallback report.
After the `cycle-report` artifacts are read back, `orchestra` must run or reuse
the configured frontend report server and automatically open the URL for that
specific cycle report in a browser. The report server is an artifact viewer only:
it must not synthesize a fallback report from the trace database. If the
configured port is occupied by an unrelated process, fail closed and report the
port conflict; do not choose an arbitrary fallback port.

If `codegen` created, modified, or deleted code, the close artifacts must include
`report.json.code_changes` with every changed hunk for that cycle. Each entry
must include the file name, language, change kind, diff hunk, changed lines, and
Korean report-layer explanation fields such as `summary_ko` or
`explanations[].text_ko`. New files are represented as added hunks containing
the full new content. Large, binary, or sensitive-looking files may be excluded
from full hunk capture only when `audit.json` records the exact exclusion reason.
Do not force explanatory Korean comments into source code only to satisfy the
report layer.

## Verbatim Evidence Trace Contract

During a work cycle, `orchestra` must append source evidence to the trace DB as
soon as each boundary event occurs. There are two separate meanings of
"automatic" in this contract:

- code-level automatic: repo code may append entries when the dev-console or
  `xavi-bootstrap trace append` CLI path is explicitly invoked;
- orchestra-protocol automatic: top-level `orchestra` must execute that append
  step every time the runtime boundary occurs.

Repo code does not, by itself, intercept Codex `spawn_agent` tool invocation
boundaries unless the surrounding runtime explicitly wires that interception.
On user prompt receipt `orchestra` stores a `user_request_verbatim` event.
Immediately before a real sub-agent spawn it must run the
`xavi-bootstrap trace append` command, or an equivalent trace append command, for
`agent_dispatch.prompt_verbatim` and confirm success before calling
`spawn_agent`. Immediately after sub-agent completion it must append the result
text verbatim when the runtime exposes it. Each verbatim event must carry
`source_ref`, `hash_sha256`, `timestamp`, `order`, `role`, and `agent_id` when an
agent id exists.

If a required runtime-boundary append fails, `orchestra` must not continue to
the spawn as if evidence exists. It must either stop before `spawn_agent` or
mark the cycle fail-closed with incomplete evidence. A summary, display string,
or later report is not a substitute for the missing verbatim append. A claim
that text is "stored in the DB" is valid only when the corresponding append
event exists with the required metadata.

`cycle-report` must never write, restore, reconstruct, or regenerate verbatim
evidence. It may only copy and verify verbatim evidence already received through
the trace DB export or explicit context bundle. If Codex runtime transcript APIs
are not readable by repo code, `orchestra` must append the spawn-before and
completion-after verbatim bundle itself. If that bundle is unavailable too, the
report must show the evidence as missing rather than invent it.

Derived projections must be named and labelled as derived data. Use field names
or UI labels containing `derived`, `summary`, or `display`. If required
verbatim evidence is absent, `cycle-report` records `audit.missing_evidence[]`,
sets `audit.status` to `fail` or `warn`, sets `evidence_status` to
`incomplete` or `fail`, and shows a visible screen warning. Missing evidence
must not be presented with success-like wording.

## No Real Sub-Agent Tool

If the runtime does not expose a real sub-agent creation tool, say so plainly.
Do not use role-separated local phases as a development fallback, and do not
claim that real sub-agents were used when the runtime only simulated roles in
one context. Report the limitation, identify the missing tool or approval point,
and stop before replacing sub-agent work with single-context implementation.
