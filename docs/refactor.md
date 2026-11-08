# Refactoring Principles

Refactoring is not primarily about making functions shorter. Its purpose is to
make the system easier to understand, change, and verify. The following
principles apply to most codebases, regardless of language or architecture.

## Model the Domain Before Extracting Helpers

A large number of small helpers often indicates that important concepts are
missing. Helpers that exist only to recover context, translate parameters, or
assemble the same object repeatedly are usually a sign that the underlying
state or boundary is wrong.

Start by naming the concepts the code is already trying to express:

- inputs and configuration
- selection and user intent
- workflow state and transitions
- external resources and their ownership

Once those concepts have explicit types, many helpers disappear because the
code no longer needs to reconstruct implicit context.

## Put Behavior Where Its Data Lives

Module boundaries should follow ownership, not directory structure. A piece of
logic belongs with the component that owns the state or invariant it depends
on. If a caller repeatedly gathers state from several places before delegating
work, the boundary is probably misplaced.

Prefer direct composition over forwarding layers. A wrapper is justified when
it adds a meaningful invariant, error boundary, lifecycle, or abstraction. A
wrapper that only renames another interface adds indirection without reducing
complexity.

## Make State Transitions Explicit

Workflows should have a small, visible set of states and transitions. A single
component should own the workflow state, and each transition should produce a
clear next state or result.

Avoid using several booleans or nullable fields to encode a mode. They create
invalid combinations and force every function to interpret the same state
differently. Prefer enums, state types, or step machines whose variants make
invalid states difficult to represent.

Remove fields that duplicate information already available elsewhere. Multiple
sources of truth eventually disagree; derived values should usually be
computed when needed rather than stored.

## Separate Core Logic From Interaction

Command-line, interactive, graphical, and network interfaces are adapters.
They collect input and present results, but they should not become the place
where core behavior is defined.

Shared logic should operate on domain inputs and return domain results. Each
adapter can then implement only the parts that genuinely differ: prompting,
formatting, progress display, cancellation, and error presentation.

This separation also makes behavior easier to test without simulating an entire
user interface.

## Treat Events as the Source of Truth

When work happens asynchronously, the event stream or authoritative state
should drive both internal decisions and user-visible progress. The UI should
project that state, not guess it from partial counters.

Keep receivers simple. If a progress view has to infer totals, phases, or task
ownership, the information is being published at the wrong boundary. Publish
the facts once and let each consumer derive its presentation.

## Deduplicate Behavior, Not Text

Extract code when the same behavior has the same meaning and is expected to
change together. Do not create an abstraction merely because two blocks look
similar today.

Good extractions preserve a concept:

- the same validation rule
- the same state transition
- the same prompt semantics
- the same external protocol

Incidental similarity is cheap to duplicate and expensive to couple.

## Design Interfaces Around Intent

Interfaces should make the valid outcomes and required decisions obvious.
Prefer domain-specific return types over ambiguous sentinels. Make cancellation,
absence, and failure explicit instead of encoding them as unrelated values.

Avoid boolean or mode parameters that switch a function between different
behaviors. Use separate functions or explicit variants when the operations have
different meanings.

Delete interfaces that no longer carry information. An unused extension point
is still a maintenance cost.

## Keep Changes Reviewable

One commit should represent one coherent idea. Separate structural refactors
from behavior changes, dependency updates, and formatting whenever practical.
This makes review, bisection, and rollback much easier.

Resolve merge conflicts by understanding both sides. Do not choose an entire
file from one branch without checking the intent behind the other branch,
especially in generated or dependency files.

## Use Metrics as Feedback, Not Targets

Formatting, linting, tests, complexity reports, and duplication reports are
useful signals. They should not dictate the design.

Optimizing directly for a metric often produces fragmented functions, shallow
wrappers, and abstractions that are harder to understand than the original
code. Fix the model first; good metrics usually follow.

## Combine Techniques Deliberately

There is no universally superior control-flow style. Choose the form that makes
the operation easiest to understand and verify:

- use loops for stateful traversal, early exits, fallible collection, and
  mutation of owned state
- use iterator chains for simple transformations and direct composition
- extract a helper only when it names a real concept or removes repeated
  behavior

Do not replace readable control flow with long `map`/`flatten`/`collect`
sequences merely to satisfy a complexity metric. A refactor that lowers CCN
while increasing token count and hiding branches in closures is not an
improvement.

Evaluate the combined result. The best refactor usually mixes techniques rather
than applying one style everywhere.

## Refactoring Checklist

- Are the important domain concepts named by explicit types?
- Is each piece of state owned by one component?
- Are invalid states difficult to represent?
- Can core behavior run without a user interface?
- Does user-visible progress derive from authoritative events?
- Is each abstraction based on shared behavior rather than visual similarity?
- Are cancellation, absence, and failure explicit?
- Does the control-flow style make the operation easier to follow?
- Did the change avoid trading a metric improvement for extra indirection?
- Can the change be reviewed and reverted as a coherent unit?
- Do formatting, linting, and relevant tests pass?
- Is the result simpler to explain than the code it replaced?
