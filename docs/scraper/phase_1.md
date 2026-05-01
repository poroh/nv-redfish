# Scraper phase 1: event-sourced runtime output

Phase 1 evolves the generic runtime from the Phase 0 step/drain API into the
first event-sourced runtime shape. The runtime still is not Redfish-specific and
must remain testable without Carbide, Redfish, HTTP, BMC mocks, generated schema
types, database crates, or application models.

Phase 1 keeps the Phase 0 target, generator, scheduled-work, work-result, and
flat round-robin scheduling concepts, but replaces the temporary Phase 0 output
and control API shape with a final-ish split between:

1. one non-cloneable runtime consumer/driver, and
2. a cloneable synchronous control handle.

The consumer receives ordered runtime output by calling `next().await`. That call
is also the runtime driver: it returns queued output immediately, otherwise it
selects and executes at most one work item, otherwise it waits to be woken by
control-plane changes or graceful shutdown.

## Product goal

Phase 1 must provide a generic runtime that can:

- create a runtime plus cloneable control handle,
- let exactly one runtime consumer receive ordered output through `next().await`,
- let cloned handles synchronously add and remove targets and generators,
- keep one ordered in-memory output queue,
- emit all work results through that queue,
- optionally emit concrete runtime control-plane events through that same queue,
- preserve causal ordering across work outputs and runtime events,
- gracefully shut down by removing all targets and draining in-flight work,
- keep target and generator id behavior from Phase 0,
- keep fallible scheduled work behavior from Phase 0,
- keep flat round-robin scheduling behavior from Phase 0 for active generators,
- keep removal safe when work is already in flight,
- support BMC-explorer-like fake discovery flow using `next().await`.

The runtime must not know what a BMC, Redfish service root, chassis, system,
endpoint report, database row, or application model is.

## Rebuild instructions for fresh Phase 1 implementation

For reproducible implementation from a clean or rolled-back tree, start from the
accepted end-of-Phase-0 scraper crate state, then apply this Phase 1 document as
the authoritative specification for the next evolution. Read the Phase 0 document
only for retained concepts and behavior explicitly listed below or not superseded
here.

When Phase 0 and Phase 1 differ, Phase 1 wins. Do not preserve Phase 0 APIs that
this document says to replace or remove, even if existing tests or historical
docs still mention them. In particular, rebuilds must remove `run_once`,
`RunOnce`, `next_output`, `drain_outputs`, the user-supplied runtime event type
parameter, and `AddGeneratorError`.

This Phase 1 document also wins over broader scraper architecture, runtime, and
requirements documents when they describe future capabilities that Phase 1 lists
as non-goals or does not explicitly require. In particular, do not implement
scheduler hierarchy, root/per-target schedulers, runtime statistics, pause/resume,
stream or batch-drain output APIs, throttling, queue pressure, lag, work-started,
or work-completed runtime events merely because they appear in broader design
documents. Those documents are roadmap context until updated after Phase 1.

A fresh implementation is complete only when the configured verification target
checks the scraper crate with both default features and `--all-features`, and
both configurations pass.

## Relationship to Phase 0

Phase 1 is an evolution of Phase 0, not a new product.

Keep these Phase 0 concepts and behavior:

- crate name and placement: `scraper/`, package `nv-redfish-scraper`,
- file headers, lint posture, and scraper Rust style requirements,
- generic runtime boundaries with no Redfish or Carbide dependencies,
- runtime-generated `TargetId` allocation from `1`, no id reuse,
- runtime-generated `GeneratorId` allocation from `1` per target, no id reuse,
- `GeneratorId::target_id()`,
- empty `TargetConfig`,
- `Readiness`,
- `Generator` trait concept,
- `ScheduledWork` as a payload-only future returning `Result<Vec<Ev>, Err>`,
- `WorkSuccess`, `WorkError`, and `WorkCompletion`,
- runtime-owned generator id enrichment for work outputs,
- `on_complete` called exactly once for executed work,
- output enqueued before completion callback,
- flat round-robin selection among active generators,
- fake-only tests.

Replace these temporary Phase 0 APIs and concepts:

- replace `Runtime::new() -> Runtime` with
  `Runtime::new() -> (Runtime, RuntimeHandle)`,
- move control APIs from `Runtime` to `RuntimeHandle`,
- replace `Runtime::run_once()` with `Runtime::next().await`,
- remove `RunOnce`,
- remove `next_output()`,
- remove `drain_outputs()`,
- remove user-supplied runtime event type parameter `R`,
- replace `AddGeneratorError` with consolidated `ControlError`.

Do not regenerate or rewrite the Phase 0 document. Phase 0 remains historical
MVP scope. Phase 1 should be implemented against this document while preserving
compatible Phase 0 concepts where this document does not supersede them.

## Crate and feature placement

Continue using the existing crate:

```text
scraper/
```

The package remains:

```toml
name = "nv-redfish-scraper"
```

Add a Cargo feature named exactly:

```toml
runtime-events = []
```

The feature must be disabled by default. Core runtime functionality must build
and test without it. Runtime event tests must be exercised with all features
enabled by the configured verification target.

If the Makefile or verification target does not already run all-features checks
for the scraper crate, update it so the scraper crate is checked with both the
default feature set and `--all-features`.

Suggested verification shape:

```text
cargo fmt -p nv-redfish-scraper
cargo build -p nv-redfish-scraper
cargo clippy -p nv-redfish-scraper
cargo test -p nv-redfish-scraper
cargo build -p nv-redfish-scraper --all-features
cargo clippy -p nv-redfish-scraper --all-features
cargo test -p nv-redfish-scraper --all-features
```

## Non-goals

Do not implement these in Phase 1:

- Redfish adapter generators,
- BMC/client construction,
- target limits,
- global limits,
- root scheduler,
- per-target scheduler,
- costs or `CostUnits`,
- classes or weights,
- WRR or DRR,
- budgets or rate limiting,
- background executor task,
- concurrent work execution,
- pause/resume APIs,
- trigger APIs,
- `run_once`,
- `run_until_idle`,
- batch drain API unless tests prove it is required,
- `Stream` implementation,
- runtime statistics,
- queue pressure events,
- readiness events,
- work-started/work-succeeded/work-failed runtime events,
- serialization features,
- durable event storage,
- application discovery policy.

If a field, function, event, or module is not used by Phase 1 behavior or tests,
do not add it.

## Public API overview

The exact module layout may be adjusted for idiomatic Rust, but Phase 1 should
expose these concepts from the crate root so tests and applications can use the
runtime without depending on private modules:

```rust
pub struct Runtime<'rt, Ev, Err>;
pub struct RuntimeHandle<'rt, Ev, Err>;

pub struct TargetConfig {}

pub struct TargetId { /* private */ }
pub struct GeneratorId { /* private */ }

pub enum ControlError {
    RuntimeShutdown,
    TargetNotFound { target_id: TargetId },
}

pub struct Readiness {
    pub ready: bool,
    pub next_ready_at: Option<std::time::Instant>,
}

pub trait Generator<'rt, Ev, Err>: Send {
    fn update_ready(&mut self, now: std::time::Instant) -> Readiness;
    fn take_next(&mut self) -> Option<ScheduledWork<'rt, Ev, Err>>;
    fn on_complete(&mut self, completion: &WorkCompletion);
}

pub struct ScheduledWork<'rt, Ev, Err> { /* private */ }

impl<'rt, Ev, Err> ScheduledWork<'rt, Ev, Err> {
    pub fn new<F>(future: F) -> Self
    where
        F: std::future::Future<Output = Result<Vec<Ev>, Err>> + Send + 'rt;
}

pub type WorkResult<Ev, Err> = Result<WorkSuccess<Ev>, WorkError<Err>>;

pub struct WorkSuccess<Ev> {
    pub generator_id: GeneratorId,
    pub events: Vec<Ev>,
}

pub struct WorkError<Err> {
    pub generator_id: GeneratorId,
    pub error: Err,
}

pub enum RuntimeOutput<Ev, Err> {
    Work(WorkResult<Ev, Err>),
    Shutdown,
    #[cfg(feature = "runtime-events")]
    Runtime(RuntimeEvent),
}

#[cfg(feature = "runtime-events")]
pub enum RuntimeEvent {
    TargetAdded { target_id: TargetId },
    GeneratorAdded { generator_id: GeneratorId },
    GeneratorRemoved { generator_id: GeneratorId },
    TargetRemoved { target_id: TargetId },
}

pub struct WorkCompletion {
    pub generator_id: GeneratorId,
    pub outcome: WorkOutcome,
}

pub enum WorkOutcome {
    Succeeded,
    Failed,
}

impl<'rt, Ev, Err> Runtime<'rt, Ev, Err> {
    pub fn new() -> (Self, RuntimeHandle<'rt, Ev, Err>);

    pub async fn next(&mut self) -> RuntimeOutput<Ev, Err>;
}

impl<'rt, Ev, Err> RuntimeHandle<'rt, Ev, Err> {
    pub fn add_target(&self, config: TargetConfig) -> Result<TargetId, ControlError>;

    pub fn remove_target(&self, target_id: TargetId) -> Result<bool, ControlError>;

    pub fn add_generator<G>(
        &self,
        target_id: TargetId,
        generator: G,
    ) -> Result<GeneratorId, ControlError>
    where
        G: Generator<'rt, Ev, Err> + 'rt;

    pub fn remove_generator(&self, generator_id: GeneratorId) -> Result<bool, ControlError>;

    pub fn graceful_shutdown(&self);
}
```

`RuntimeHandle` must implement `Clone`. `Runtime` must not implement `Clone`.
`Runtime::next` takes `&mut self` so the single consumer is enforced by the
borrow checker.


## Event sourcing model

Phase 1 uses "event sourcing" to mean an ordered, in-memory runtime output log
that applications consume through `Runtime::next().await`.

MVP behavior:

- one runtime has one output queue,
- the queue is FIFO,
- the queue is unbounded in Phase 1,
- all completed work appears as `RuntimeOutput::Work`,
- graceful termination appears as `RuntimeOutput::Shutdown`,
- runtime control-plane events appear as `RuntimeOutput::Runtime` only when the
  `runtime-events` feature is enabled,
- output order is the runtime's causal order,
- applications own durable persistence, replay, projections, and database writes.

Durability is explicitly not a runtime responsibility in Phase 1. If an
application wants durable event storage, it must persist items returned by
`next().await` itself.

## Runtime output

### `RuntimeOutput`

```rust
pub enum RuntimeOutput<Ev, Err> {
    Work(WorkResult<Ev, Err>),
    Shutdown,
    #[cfg(feature = "runtime-events")]
    Runtime(RuntimeEvent),
}
```

MVP behavior:

- `Work` contains successful or failed work output,
- `Shutdown` marks graceful runtime termination,
- `Runtime` exists only with `runtime-events`,
- there is no user-supplied runtime event type parameter,
- with default features, `RuntimeOutput` has only `Work` and `Shutdown`,
- with `runtime-events`, `RuntimeOutput` has `Work`, `Shutdown`, and `Runtime`,
- Phase 1 runtime code must not emit any runtime event except target/generator
  added/removed events.

`Shutdown` is not a runtime event. Do not also emit a runtime event for graceful
shutdown.

### Work output

Work output behavior remains the Phase 0 behavior:

- scheduled work returns payload-only `Result<Vec<Ev>, Err>`,
- runtime awaits scheduled work,
- runtime attaches `generator_id`,
- runtime constructs `WorkSuccess<Ev>` or `WorkError<Err>`,
- runtime enqueues `RuntimeOutput::Work(result)`,
- events inside `WorkSuccess.events` preserve the order returned by the future,
- a work failure still produces `RuntimeOutput::Work(Err(...))`.

Do not add separate runtime events for work started, work succeeded, or work
failed in Phase 1. Work success and failure are already represented by `Work`.

### Runtime events

Runtime events are concrete runtime-owned events compiled only with the
`runtime-events` feature:

```rust
#[cfg(feature = "runtime-events")]
pub enum RuntimeEvent {
    TargetAdded { target_id: TargetId },
    GeneratorAdded { generator_id: GeneratorId },
    GeneratorRemoved { generator_id: GeneratorId },
    TargetRemoved { target_id: TargetId },
}
```

MVP behavior:

- emitted after the corresponding successful state mutation or removal
  finalization,
- emitted into the same ordered queue as work outputs,
- contain runtime ids only,
- never contain user work events,
- never contain user work errors,
- never contain application model data,
- not emitted when the `runtime-events` feature is disabled,
- emission code should not be compiled when the feature is disabled,
- disabled-feature builds must not use fake output-queue side effects or
  placeholder mutations solely to satisfy lints; use clean cfg-gating or true
  no-op helpers instead.

`GeneratorAdded` and `GeneratorRemoved` carry only `generator_id`; recover the
parent target with `generator_id.target_id()` when needed.

## Runtime and handle split

### `Runtime::new`

```rust
pub fn new() -> (Runtime<'rt, Ev, Err>, RuntimeHandle<'rt, Ev, Err>);
```

MVP behavior:

- creates shared runtime state,
- initializes id counters,
- initializes empty target and generator maps,
- initializes the flat round-robin scheduler,
- initializes an empty output queue,
- initializes wake state for `next().await`,
- returns one non-cloneable `Runtime` consumer,
- returns one cloneable `RuntimeHandle` control handle.

No configuration is accepted in Phase 1.

### `Runtime`

`Runtime` is the only output consumer and runtime driver.

MVP behavior:

- is not `Clone`,
- exposes `next(&mut self).await`,
- does not expose control-plane mutation APIs,
- does not expose `run_once`, `next_output`, or `drain_outputs`,
- owns no application policy.

### `RuntimeHandle`

`RuntimeHandle` is the cloneable synchronous control plane.

MVP behavior:

- implements `Clone`,
- exposes target and generator mutation APIs,
- exposes graceful shutdown request API,
- does not expose `next`,
- methods are synchronous,
- methods may briefly block on internal runtime-state locking,
- methods must not wait for work futures,
- methods must not wait for in-flight work to complete.

## Control errors

Phase 1 uses one consolidated control error enum:

```rust
pub enum ControlError {
    RuntimeShutdown,
    TargetNotFound { target_id: TargetId },
}
```

MVP behavior:

- `RuntimeShutdown` is returned by mutating control APIs after graceful shutdown
  has started,
- `TargetNotFound { target_id }` is returned when adding a generator under a
  target that does not exist or is already logically removed,
- remove APIs still return `Ok(false)` when the requested active object does not
  exist or is already removing,
- failed synchronous control operations do not emit runtime events.

Do not add many operation-specific error enums in Phase 1.


## Target API

### `TargetConfig`

```rust
pub struct TargetConfig {}
```

MVP behavior remains Phase 0 behavior:

- empty configuration,
- no target limits,
- no concurrency settings,
- no debug name,
- no scheduling weights.

### `RuntimeHandle::add_target`

```rust
pub fn add_target(&self, config: TargetConfig) -> Result<TargetId, ControlError>;
```

MVP behavior:

- returns `Err(ControlError::RuntimeShutdown)` if graceful shutdown has started,
- allocates a new `TargetId`,
- creates target state in the runtime,
- initializes that target's generator sequence,
- returns the new id,
- if `runtime-events` is enabled, enqueues
  `RuntimeOutput::Runtime(RuntimeEvent::TargetAdded { target_id })` after the
  target exists,
- wakes the runtime consumer if an output was enqueued.

The function does not fail for any reason other than graceful shutdown in Phase
1.

### `RuntimeHandle::remove_target`

```rust
pub fn remove_target(&self, target_id: TargetId) -> Result<bool, ControlError>;
```

Phase 1 removal is a removal request. Removal is complete when the corresponding
runtime events have been consumed, not necessarily when the control function
returns.

MVP behavior:

- returns `Err(ControlError::RuntimeShutdown)` if graceful shutdown has started,
- returns `Ok(false)` if the target does not exist or is already removing,
- marks the target logically removed/removing immediately,
- makes the target unavailable to future `add_generator` calls immediately,
- marks every generator under the target as removing,
- removes those generators from the flat scheduler immediately,
- ensures those generators are never queried for readiness again,
- does not cancel in-flight work,
- does not wait for in-flight work,
- returns `Ok(true)` after the removal request is recorded,
- if no child generator has in-flight work, finalizes child removals immediately,
- finalizes child generator removals in deterministic target generator order,
- if a later child drains before an earlier child, holds the later removal event
  until earlier child removals have finalized,
- if `runtime-events` is enabled, emits one `GeneratorRemoved` event for each
  child generator in deterministic target generator order,
- if `runtime-events` is enabled, emits `TargetRemoved` only after all child
  generator removals for that target have finalized,
- wakes the runtime consumer if output was enqueued or removal state changed.

`remove_target` behaves as if each child generator was requested for removal by
user control flow. The target removal event is causally after all child generator
removal events.

## Generator API

The `Generator` trait remains the Phase 0 trait:

```rust
pub trait Generator<'rt, Ev, Err>: Send {
    fn update_ready(&mut self, now: std::time::Instant) -> Readiness;
    fn take_next(&mut self) -> Option<ScheduledWork<'rt, Ev, Err>>;
    fn on_complete(&mut self, completion: &WorkCompletion);
}
```

MVP behavior:

- active generators can be queried for readiness,
- removing generators must not be queried for readiness,
- removing generators must not be selected for new work,
- a generator with in-flight work must be retained until completion is reported,
- `on_complete` is called exactly once for each selected work item even if
  removal was requested while that work was in flight,
- generator callbacks must not call runtime control APIs reentrantly; application
  policy should react to consumed `RuntimeOutput` values instead.

### `RuntimeHandle::add_generator`

```rust
pub fn add_generator<G>(
    &self,
    target_id: TargetId,
    generator: G,
) -> Result<GeneratorId, ControlError>
where
    G: Generator<'rt, Ev, Err> + 'rt;
```

MVP behavior:

- returns `Err(ControlError::RuntimeShutdown)` if graceful shutdown has started,
- verifies that `target_id` exists and is not removing,
- returns `Err(ControlError::TargetNotFound { target_id })` if the target does
  not exist or is already removing,
- allocates a new `GeneratorId` under that target,
- stores the boxed generator,
- records the generator under the target,
- inserts the generator id into the flat round-robin scheduler,
- returns the new generator id,
- if `runtime-events` is enabled, enqueues
  `RuntimeOutput::Runtime(RuntimeEvent::GeneratorAdded { generator_id })`
  after the generator is active,
- wakes the runtime consumer.

### `RuntimeHandle::remove_generator`

```rust
pub fn remove_generator(&self, generator_id: GeneratorId) -> Result<bool, ControlError>;
```

Phase 1 generator removal is a removal request. Removal is complete when
`GeneratorRemoved` has been emitted and consumed when runtime events are enabled.
When runtime events are disabled, removal is still finalized internally after any
in-flight work drains.

MVP behavior:

- returns `Err(ControlError::RuntimeShutdown)` if graceful shutdown has started,
- returns `Ok(false)` if the generator does not exist or is already removing,
- marks the generator as removing immediately,
- removes the generator from the flat scheduler immediately,
- removes the generator from the active list of its parent target immediately,
- ensures the generator is never queried for readiness again,
- ensures the generator cannot create new work,
- does not cancel in-flight work,
- does not wait for in-flight work,
- returns `Ok(true)` after the removal request is recorded,
- retains the generator object while it has in-flight work so completion can be
  reported exactly once,
- finalizes removal immediately if no work is in flight,
- finalizes removal after completion if work is in flight,
- if `runtime-events` is enabled, enqueues
  `RuntimeOutput::Runtime(RuntimeEvent::GeneratorRemoved { generator_id })`
  when removal finalizes,
- wakes the runtime consumer if output was enqueued or removal state changed.

A second `remove_generator` call for a generator already marked removing returns
`Ok(false)`.


## Scheduled work and in-flight behavior

Scheduled work remains the Phase 0 payload-only future:

```rust
pub struct ScheduledWork<'rt, Ev, Err> { /* private */ }

impl<'rt, Ev, Err> ScheduledWork<'rt, Ev, Err> {
    pub fn new<F>(future: F) -> Self
    where
        F: std::future::Future<Output = Result<Vec<Ev>, Err>> + Send + 'rt;
}
```

MVP behavior:

- runtime selects at most one work item per `next().await` call,
- runtime marks the selected generator as in flight before awaiting the work,
- runtime must not hold the runtime-state lock while awaiting work,
- control APIs may run while work is being awaited,
- removal requested during in-flight work does not cancel that work,
- when work completes, runtime enqueues `RuntimeOutput::Work(...)`,
- runtime calls `on_complete` exactly once on the originating generator,
- runtime decrements in-flight state,
- runtime finalizes deferred generator/target removal if in-flight state reaches
  zero,
- work output is causally before removal events caused by removal waiting for
  that work to drain.

Ordering for a generator removed while work is in flight:

1. work is selected,
2. generator becomes in flight,
3. user requests generator removal,
4. generator is removed from scheduler and active target membership,
5. work completes,
6. `RuntimeOutput::Work(...)` is enqueued,
7. `on_complete` is called,
8. generator removal finalizes,
9. if `runtime-events` is enabled, `GeneratorRemoved` is enqueued.

## `Runtime::next`

```rust
pub async fn next(&mut self) -> RuntimeOutput<Ev, Err>;
```

`next` is both the output consumer and the runtime driver.

MVP behavior:

1. If the runtime has already returned `Shutdown`, return `Shutdown` forever.
2. If the output queue has an item, pop and return the oldest item immediately.
3. If graceful shutdown is complete and no queued item remains, return
   `Shutdown` and remember that shutdown was returned.
4. Otherwise, scan active generators in flat round-robin order.
5. Skip candidate ids that no longer exist or are removing.
6. Call `update_ready(now)` on active candidates.
7. Skip generators that are not ready.
8. Call `take_next` on the first ready candidate.
9. If `take_next` returns `None`, continue scanning during the same `next` call.
10. If a work item is returned, mark the generator in flight, release the
    runtime-state lock, await the work, reacquire runtime state, enqueue
    `RuntimeOutput::Work(...)`, call `on_complete`, finalize deferred removals,
    then return the oldest queued output.
11. If no generator produces work and no output is queued, wait indefinitely for a
    wake notification.
12. Wake notifications come from control-plane changes, output enqueues,
    graceful shutdown, and future timer/control triggers.

Phase 1 has no timer support. A runtime with no queued output, no ready
work, and no future control-plane wake will wait indefinitely in `next().await`.
This is expected Phase 1 behavior.

`next().await` must not spin, busy-poll, or otherwise run a tight CPU loop when
no output is queued and a scheduler scan finds no executable work. After one
full scan that produces no work, the runtime must park the caller's future and
return `Poll::Pending` until an actual wake source occurs. The mere presence of
active generators in scheduler state is not itself a wake source, because those
generators may be not ready or may return `None` from `take_next`.

The wake mechanism should behave like a single-consumer async condition
variable, not like a blocking `std::sync::Condvar` on an executor thread. A
recommended simple design is:

1. keep runtime state behind the existing synchronous mutex,
2. store the current task `Waker` when `next` cannot make progress,
3. keep a monotonically increasing wake generation/epoch in runtime state,
4. increment that generation after every control-plane state mutation that may
   make `next` worth polling again,
5. wake the stored waker after releasing runtime state,
6. have `next` remember the generation observed after a no-work scan,
7. return `Poll::Pending` until queued output exists, shutdown is complete, or
   the generation has changed.

The wake-generation and stored-waker logic must be isolated in a small private
helper type instead of being spread directly across `RuntimeState`,
`Runtime::next` future polling code, and `RuntimeHandle` methods. The helper
should own the single-consumer wake state, including the current stored `Waker`
and monotonically increasing generation/epoch, and expose focused operations for
observing the generation, registering the current task waker, and marking that a
state mutation may make `next` worth polling again. Runtime code should interact
with wake state through that helper so the lost-wake protocol is reviewable in
one place.

Waking after releasing runtime state is a Phase 1 requirement, not just an
optimization. Do not call user or executor wakers while holding the runtime-state
mutex unless a later document explicitly changes this rule.

The implementation must avoid lost wakes. Do not check the wait condition,
release state, and only then store the waker; a control-plane change could occur
in that gap and leave `next` asleep forever. Store or update the waker in an
ordering that is protected by the same state change protocol as wake-generation
updates, then re-check or otherwise prove that queued output, shutdown
completion, and generation changes cannot be missed.

`next` must never return an `Idle` item or `Option::None` in Phase 1.
Graceful shutdown is represented by `RuntimeOutput::Shutdown`.

## Graceful shutdown

### `RuntimeHandle::graceful_shutdown`

```rust
pub fn graceful_shutdown(&self);
```

Graceful shutdown is a request to remove every target and terminate after all
in-flight work drains.

MVP behavior:

- idempotent,
- first call starts graceful shutdown,
- later calls do nothing,
- once graceful shutdown starts, all mutating control APIs return
  `Err(ControlError::RuntimeShutdown)`,
- no new targets may be added,
- no new generators may be added,
- no explicit target/generator removal requests are accepted,
- all active targets are marked removing,
- all active generators are marked removing,
- all removing generators are removed from the scheduler immediately,
- no new work is selected after shutdown starts,
- already in-flight work is allowed to complete,
- work outputs for already in-flight work are enqueued before related removal
  events,
- if `runtime-events` is enabled, generator removal events are emitted as
  generator removals finalize,
- if `runtime-events` is enabled, target removal events are emitted after their
  child generator removals finalize,
- when zero targets remain, no work is in flight, and all prior queued output has
  been returned, `next().await` returns `RuntimeOutput::Shutdown`,
- after `Shutdown` has been returned once, every later `next().await` returns
  `Shutdown` immediately.

If graceful shutdown starts when there are no targets, the next `next().await`
returns `Shutdown` immediately unless older queued output must be returned first.

Do not emit a runtime event for graceful shutdown requested or completed.

## Internal architecture

Phase 1 runtime should use an internal shape equivalent to:

```text
Runtime<'rt, Ev, Err>
  shared: Arc<RuntimeShared<'rt, Ev, Err>>
  not Clone

RuntimeHandle<'rt, Ev, Err>
  shared: Arc<RuntimeShared<'rt, Ev, Err>>
  Clone

RuntimeShared
  state: Mutex<RuntimeState<'rt, Ev, Err>>

WakeState or equivalent private helper
  generation: u64
  waiter: Option<std::task::Waker>
  observes the current generation
  registers the current task waker
  marks state changes and returns a waker to wake after unlock

RuntimeState
  next_target_id: u64
  targets: HashMap<TargetId, TargetState>
  generators: HashMap<GeneratorId, GeneratorSlot<'rt, Ev, Err>>
  scheduler: FlatRoundRobin
  outputs: VecDeque<RuntimeOutput<Ev, Err>>
  shutting_down: bool
  shutdown_returned: bool

TargetState
  next_generator_id: u64
  active_generators: Vec<GeneratorId>
  removing_generators: Vec<GeneratorId> or equivalent deterministic state
  removing: bool

GeneratorSlot<'rt, Ev, Err>
  generator: Box<dyn Generator<'rt, Ev, Err> + 'rt>
  in_flight: usize
  removing: bool
```

Keep `RuntimeHandle` methods thin. Public handle methods should do little more
than acquire runtime state, delegate the actual mutation to focused private
`RuntimeState` helpers, collect any waker returned by wake-state mutation, release
runtime state, and wake after unlock. This keeps synchronous control-plane APIs
simple and keeps mutation semantics reviewable in one place.

Use focused private helpers for output construction and enqueueing when that
improves ordering clarity. In particular, work-result enrichment should remain a
runtime responsibility, but the conversion from `(generator_id, Result<Vec<Ev>,
Err>)` to `RuntimeOutput::Work(...)` plus `WorkOutcome` may be isolated in helper
functions so the required "enqueue output before `on_complete`" order is easy to
audit. Runtime-event enqueueing should similarly be isolated behind cfg-gated
helpers when `runtime-events` is enabled.

Removal bookkeeping is the next most complex internal behavior after wake
handling. Implementations should keep deterministic target/generator removal
ordering, in-flight drain checks, and target-finalization decisions behind focused
private helpers or small private tracker types when doing so makes the code easier
to reason about. Do not expose these helpers publicly and do not overbuild them
into a general scheduler or lifecycle framework.

Exact field names and helper types are implementation details.

Runtime state is fully synchronous. Scheduler operations, target/generator map
updates, readiness checks, and future generation through `take_next` happen while
holding short-lived runtime state access. Awaiting the selected work future must
happen after releasing runtime state access.

Because there is only one `Runtime` consumer, the wake mechanism may be designed
for a single waiter. The runtime implementation should avoid forcing a Tokio
runtime. An executor-agnostic waker implementation is preferred. A small generic
futures dependency is acceptable if needed, but do not add broad async runtime
dependencies to the scraper library for Phase 1.


## Flat round-robin scheduler

The flat scheduler remains synchronous and generic.

It must:

- store active generator ids in insertion order,
- maintain round-robin cursor state,
- support insertion of a generator id,
- support removal of a generator id,
- provide candidates for a `next` scan,
- be deterministic.

It must not:

- call generators,
- inspect readiness,
- create work,
- await futures,
- enqueue output,
- know runtime event payloads,
- know Redfish or application semantics,
- know target limits or costs.

Required scan behavior:

1. `next` scans at most the active generators that are present in scheduler state
   when the scan starts.
2. The scheduler returns candidates in flat global round-robin order.
3. The scheduler advances the cursor as candidates are considered, including
   candidates that are not ready or whose `take_next` returns `None`.
4. If a candidate produces work, the next scan starts after that candidate.
5. If no candidate produces work, the next scan starts after the full scan.
6. Removed/removing generator ids are deleted from scheduler state and are never
   returned again.

## Output ordering requirements

Phase 1 ordering is part of the public behavior.

Required ordering:

- FIFO queue order is preserved by `next().await`,
- work events inside one `WorkSuccess.events` preserve future-returned order,
- control-plane runtime events are emitted after successful state mutation,
- `TargetAdded` is after the target exists,
- `GeneratorAdded` is after the generator is active and schedulable,
- `GeneratorRemoved` is after the generator is no longer schedulable and any
  in-flight work has completed,
- `TargetRemoved` is after all child generator removals have finalized,
- if work is in flight when removal is requested, the work output is before the
  corresponding removal runtime event,
- `Shutdown` is after all prior queued output has been returned.

When the `runtime-events` feature is disabled, the same internal causal ordering
must still be honored for work and shutdown. Runtime event items simply are not
compiled or emitted.

## BMC-explorer-like Phase 1 test

Phase 1 must retain the fake discovery-flow test from Phase 0, updated to use
`Runtime::new()` returning a consumer and handle, and `next().await` for output.

The test should model this flow:

1. Application creates `(runtime, handle)` for
   `Runtime<FakeExplorerEvent, FakeExplorerError>`.
2. Application adds one or more targets through the handle.
3. Application adds an initial service-root generator under each target.
4. Application calls `runtime.next().await` until it receives the service-root
   work output.
5. Application-owned policy inspects the fake service-root event.
6. Application adds fake system and chassis generators under the same target
   through the handle.
7. More `next().await` calls emit fake system/chassis discovery work outputs.
8. Application builds a fake exploration report externally from consumed outputs.

The final report must be built from runtime outputs, preserving deterministic
output order. The runtime must not build the report.

Do not include target ids inside fake events just to satisfy runtime needs; the
runtime-provided `WorkSuccess` and `WorkError` already carry generator identity,
and target identity is available through `generator_id.target_id()`.

When this test is compiled with `runtime-events`, it may either consume and
ignore runtime control-plane events or assert their ordering explicitly. It must
not rely on runtime events being available in the default feature set.

## Required tests

All tests must use fake generators, fake events, and fake errors only. At least
one test must use event and error payload types that intentionally do not
implement `Clone`, `Debug`, `Eq`, or `PartialEq`, so the test suite catches
accidental trait bounds on user payloads.

Keep Phase 0 behavioral coverage where applicable, updated to the Phase 1 API.

Test organization is a Phase 1 requirement, not an implementation detail:

- Do not collapse scraper integration tests into a single phase file such as
  `phase_1.rs`.
- Preserve or restore behavior-domain test files, updating them in place for the
  Phase 1 API. Expected files include `ids.rs`, `control.rs`, `scheduling.rs`,
  `output.rs`, `completion.rs`, `discovery_flow.rs`, and shared helpers under
  `common/` when helpers are useful.
- Add new tests to the behavior-domain file that matches the behavior being
  tested. For example, add graceful shutdown and removal tests to control-focused
  coverage, runtime output ordering tests to output-focused coverage, scheduling
  tests to scheduling-focused coverage, and fake discovery-flow coverage to
  `discovery_flow.rs`.
- A rebuild is incomplete if existing domain test files are deleted merely to
  make the Phase 1 API migration easier.
- Consolidating tests into one file is allowed only after explicit human approval
  in the current session.

Required test scenarios should be placed as follows unless a more specific
behavior-domain split is approved:

- `ids.rs`: runtime construction, handle cloning, target id generation, and
  generator id parent/display behavior.
- `control.rs`: target/generator add/remove errors, request-based removals,
  in-flight removal safety, graceful shutdown rejection and draining behavior,
  and `next` wakeups caused by control-plane changes.
- `scheduling.rs`: flat round-robin order, readiness skipping, and `take_next`
  only being called for selected generators.
- `output.rs`: FIFO output behavior, work success/error wrapping, per-work event
  ordering, runtime output feature-gating, and payload types without common
  trait bounds.
- `completion.rs`: success and failure completion callback behavior.
- `discovery_flow.rs`: the fake BMC-explorer-like discovery flow.
- Runtime-event-specific assertions may live in the relevant behavior-domain file
  behind `#[cfg(feature = "runtime-events")]` or in a clearly named
  runtime-event test file if that keeps the behavior-domain files readable.


### Test: runtime construction returns consumer and handle

- Create `(runtime, handle)`.
- Verify the handle can be cloned.
- Verify the runtime is used as the sole consumer through `next().await` in other
  tests.

### Test: target ids are generated

- Add two targets through the handle.
- Verify ids are distinct.
- Verify display strings are `target #1` and `target #2`.

### Test: generator ids include target ids

- Add two targets.
- Add generators under each target.
- Verify `generator_id.target_id()` returns the parent target.
- Verify display strings such as `generator #1.1`, `generator #1.2`, and
  `generator #2.1`.

### Test: add generator requires existing target

- Create a target and request its removal, or use an id from a missing target.
- Attempt to add a generator.
- Verify `ControlError::TargetNotFound { target_id }` is returned.

### Test: flat round-robin order through `next`

- Add one target.
- Add three always-ready generators A, B, and C.
- Each generator returns one event per selection.
- Repeated `next().await` calls should produce work events in order:

```text
A, B, C, A, B, C
```

- If runtime events are enabled in this test, consume or filter control-plane
  runtime events without disturbing work ordering assertions.

### Test: output event ordering inside one work item

- Add one generator whose work returns multiple events.
- Call `next().await` until the work output is received.
- Verify events inside `WorkSuccess.events` preserve returned order.

### Test: work error output

- Add one generator whose work returns `Err(FakeExplorerError)`.
- Call `next().await` until the work output is received.
- Verify output is `RuntimeOutput::Work(Err(WorkError { ... }))`.
- Verify `WorkError` contains runtime-provided generator id.

### Test: completion callback on success

- Add one successful generator.
- Call `next().await` until work output is received.
- Verify the generator observed exactly one completion with
  `WorkOutcome::Succeeded`.

### Test: completion callback on failure

- Add one failing generator.
- Call `next().await` until work output is received.
- Verify the generator observed exactly one completion with
  `WorkOutcome::Failed`.

### Test: not-ready generator is skipped

- Add one not-ready generator and one ready generator.
- Call `next().await` until work output is received.
- Verify `take_next` was not called on the not-ready generator.
- Verify ready generator ran.

### Test: work is created only when selected

- Add multiple generators.
- Count `take_next` calls.
- Verify a generator's `take_next` count increases only when selected, not when
  added and not when another generator runs.


### Test: remove generator is a request

- Add generator A whose work can be in flight and generator B.
- Request removal of B when B has no in-flight work.
- Verify `remove_generator` returns `Ok(true)`.
- Verify B is never queried for readiness again.
- With `runtime-events`, verify `GeneratorRemoved` is emitted.

### Test: repeated remove generator while removing returns false

- Add a generator whose work is in flight.
- Request removal.
- Request removal again before finalization.
- Verify the first call returns `Ok(true)` and the second returns `Ok(false)`.

### Test: remove target removes generators in order

- Add target T with generators A and B.
- Request target removal.
- Verify A and B are never queried for readiness again.
- With `runtime-events`, verify `GeneratorRemoved(A)` then
  `GeneratorRemoved(B)` then `TargetRemoved(T)`.

### Test: in-flight work completes before removal event

- Add one generator whose selected work waits on a test-controlled future.
- Start `next().await` so the work is in flight.
- Request generator removal through a cloned handle.
- Complete the work.
- Verify the work output is returned before `GeneratorRemoved` when runtime
  events are enabled.
- Verify `on_complete` is called exactly once.

### Test: target removal waits internally for in-flight child removal

- Add a target with one generator whose selected work is in flight.
- Request target removal.
- Complete the work.
- With `runtime-events`, verify work output precedes `GeneratorRemoved`, and
  `GeneratorRemoved` precedes `TargetRemoved`.

### Test: add generator to removing target fails

- Add a target.
- Request target removal.
- Attempt to add a generator under that target.
- Verify `ControlError::TargetNotFound { target_id }`.

### Test: runtime events are feature gated

Under default features:

- `RuntimeOutput::Runtime` and `RuntimeEvent` are not compiled.
- Work outputs and shutdown still compile and pass tests.

Under `--all-features`:

- target and generator add/remove runtime events are emitted in deterministic
  order.

### Test: target and generator added runtime events

With `runtime-events` enabled:

- Add target.
- Verify `TargetAdded` is returned by `next().await` before later outputs.
- Add generator.
- Verify `GeneratorAdded` is returned with the generator id.

### Test: graceful shutdown with no targets

- Create runtime and handle.
- Call `handle.graceful_shutdown()`.
- Verify the next `runtime.next().await` returns `RuntimeOutput::Shutdown`.
- Verify later `next().await` calls return `Shutdown` forever.

### Test: graceful shutdown rejects later control mutations

- Call `graceful_shutdown`.
- Verify `add_target`, `add_generator`, `remove_target`, and `remove_generator`
  return `Err(ControlError::RuntimeShutdown)` where applicable.

### Test: graceful shutdown removes targets and generators

- Add targets and generators.
- Call `graceful_shutdown`.
- With `runtime-events`, verify generator removals, target removals, then
  `Shutdown` after prior outputs are consumed.
- Without runtime events, verify `Shutdown` is eventually returned after in-flight
  work drains.

### Test: next waits and wakes on control-plane work

- Start `runtime.next().await` with no queued outputs and no generators.
- From a cloned handle, add a target and ready generator.
- Verify the waiting `next` call wakes and eventually returns output.
- If `runtime-events` is enabled, account for target/generator added events in
  ordering.

### Test: BMC-explorer-like discovery flow

- Implement the fake discovery flow described above.
- Build the fake exploration report in test/application code from consumed
  runtime outputs.
- Verify deterministic output ordering.

## Implementation guardrails

Follow the common scraper Rust style guide in [rust-style-guide.md](rust-style-guide.md).
Phase 1 also has these specific constraints:

- Keep the runtime generic over lifetime `'rt`, `Ev`, and `Err`.
- Do not keep or add a user-supplied runtime event payload type parameter `R`.
- Keep runtime events concrete and behind `runtime-events`.
- Keep `runtime-events` disabled by default.
- Keep target config empty.
- Keep one non-cloneable runtime consumer.
- Keep the handle cloneable.
- Keep control APIs synchronous.
- Keep runtime state operations synchronous and short-lived.
- Do not hold runtime state locks while awaiting scheduled work.
- Do not add pause/resume.
- Do not add trigger APIs.
- Do not add `run_once`.
- Do not add `run_until_idle`.
- Do not add cost/class/budget fields.
- Do not add target limits.
- Do not spawn background runtime driver tasks.
- Do not implement `Stream` in Phase 1.
- Do not make the scheduler async.
- Do not let scheduled work construct `WorkSuccess` or `WorkError`.
- Runtime must attach generator ids to work outputs.
- Runtime must preserve output order.
- Removing generators must not be queried for readiness again.
- Removing targets must mark all their generators removing.
- In-flight work must complete and report completion exactly once.
- Removal requests must not wait for in-flight work.
- Graceful shutdown must drain in-flight work and then return `Shutdown`.
- Avoid accidental implementation artifacts such as duplicate wake calls, no-op
  queue mutations, placeholder side effects, or helper APIs that exist only to
  appease lints.
- Do not add Redfish or Carbide dependencies.


## Architecture document follow-up

After Phase 1 is implemented and accepted, update broader architecture/runtime
documents to reflect the new runtime shape. Do not rewrite Phase 0.

Expected follow-up edits:

- update [architecture.md](architecture.md) to mention the runtime consumer plus
  cloneable handle split,
- update [runtime.md](runtime.md) to remove the user-supplied runtime event type
  parameter `R`,
- update [runtime.md](runtime.md) to describe `Runtime::new() -> (Runtime,
  RuntimeHandle)`,
- update [runtime.md](runtime.md) to describe `next().await` as the output
  consumer and driver,
- update [runtime.md](runtime.md) to describe graceful shutdown,
- update [runtime.md](runtime.md) to clarify that runtime durability is an
  application responsibility,
- update [runtime.md](runtime.md) to distinguish Phase 1 runtime events from
  future observability/statistics events.

The existing architecture and requirements documents contain future requirements
such as hierarchy, stats, queue pressure, lag, throttling, work-started events,
and stream APIs. Those remain future scope unless this Phase 1 document
explicitly requires them.

## Implementation workflow

After the initial implementation compiles, do two explicit review/fix cycles
against this phase document before considering Phase 1 complete:

1. Run the configured verification target.
2. Review pass 1 against `docs/scraper/phase_1.md`:
   - compare the public API to the documented API,
   - compare runtime behavior to each MVP behavior section,
   - compare output ordering to the ordering requirements,
   - compare tests to the required tests,
   - compare the implementation to the guardrails,
   - fix any gaps found in the review.
3. Run the configured verification target again.
4. Review pass 2 against `docs/scraper/phase_1.md`:
   - focus on missed edge cases,
   - inspect in-flight removal and graceful shutdown behavior,
   - remove overbuilt or placeholder APIs,
   - check for overbuilt APIs introduced by pass 1 fixes,
   - check fake test quality and deterministic ordering,
   - check that tests remain split by behavior domain and were not collapsed into
     a single phase file,
   - check that every required test scenario in this document is present, not
     merely that the current tests pass,
   - check for accidental artifacts such as duplicate wake calls or fake no-op
     mutations introduced to satisfy lints,
   - fix any gaps found in the review.
5. Run the configured verification target one final time.
6. Summarize completion only after both review passes and final verification are
   done.

The review passes must compare the implementation against this document, not
only against compiler, formatter, clippy, or test output.

## Completion criteria

Phase 1 is complete when:

- `Runtime::new()` returns a non-cloneable `Runtime` and cloneable
  `RuntimeHandle`,
- control APIs exist only on `RuntimeHandle`,
- `Runtime::next(&mut self).await` is the output consumer and driver,
- `run_once`, `RunOnce`, `next_output`, and `drain_outputs` are removed,
- `RuntimeOutput` contains `Work` and `Shutdown` by default,
- `RuntimeOutput::Runtime(RuntimeEvent)` exists only with `runtime-events`,
- no user-supplied runtime event type parameter remains,
- runtime events are concrete and limited to target/generator added/removed,
- runtime events are emitted only with `runtime-events`,
- target and generator ids still behave as specified,
- generator ids still expose parent target ids,
- flat round-robin scheduling still works for active generators,
- `next` executes at most one work item per call,
- work success and work failure both appear as ordered work outputs,
- runtime constructs `WorkSuccess` and `WorkError`,
- completion callbacks are called once per executed work item,
- removing generators and targets is request-based and safe with in-flight work,
- removing generators are never queried for readiness again,
- target removal emits/finalizes child generator removals before target removal,
- graceful shutdown drains in-flight work and then returns `Shutdown`,
- after `Shutdown`, later `next` calls return `Shutdown` forever,
- the fake BMC-explorer-like discovery-flow test passes with the Phase 1 API,
- required tests exist in behavior-domain integration test files, not in a single
  phase catch-all file, unless that consolidation was explicitly approved by the
  human in the current session,
- every required test scenario in this document has been audited against the test
  suite; passing `cargo test` alone is not sufficient if required scenarios are
  missing,
- default-feature and all-features configured checks pass,
- no unused scheduler/cost/limit/statistics APIs are present.

## Next phase preview

A later phase should introduce timeline-aware runtime behavior. All operations
that need deterministic time should accept or derive testable `Instant` values so
tests can run at CPU speed without timing sleeps. Timer wakeups, periodic
readiness, control-plane trigger generators, hierarchy, target limits, and
statistics remain future work.

Redfish adapter work should wait until the generic runtime boundary is stable
enough for adapter generators to consume.
