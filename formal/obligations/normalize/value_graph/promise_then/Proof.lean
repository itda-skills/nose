/-
Soundness of nose's local Promise continuation canonicalization.

The value graph already strips `await e` to its operand `e`, i.e. it models a *settled* Promise
by its resolved value (an eager / identity-monad model — sound for clone detection, where the
two forms compute the same result and the scheduling difference is irrelevant). Under that model
`p.then(λr. body r)` is just the continuation applied to the resolved value, which is exactly
what `let r = await p; body r` computes. So the value graph beta-reduces the `.then` callback
over the receiver (`eval_promise_then`), and a `.then`-chain is the nested await composition.

The implementation also tracks a local fulfilled/rejected channel model for first-party
`Promise.reject`, `.catch`, and two-argument `.then`. That model is deliberately smaller than
JavaScript's full Promise semantics: it only covers dependency-closed local producers and
handlers, and it keeps thenable assimilation, `.finally`, aggregates, and scheduling closed.

This file proves those two equalities for the model. Self-contained; checked by the formal
obligation CI gate.
-/

namespace NosePromiseThen

/-- `await p` on a settled promise: the value graph models a settled `Promise α` by its resolved
    value `α`, so awaiting is the identity. -/
def awaitP (p : α) : α := p

/-- `p.then(f)` on a settled promise: run the continuation on the resolved value. -/
def thenP (p : α) (f : α → β) : β := f p

/-- The rewrite `eval_promise_then` performs: `p.then(λr. body r)` ≡ `let r = await p; body r`.
    Beta-reducing the callback over the receiver is meaning-preserving. -/
theorem then_eq_await_bind (p : α) (f : α → β) : thenP p f = f (awaitP p) := rfl

/-- A `.then`-chain is the nested sequential composition of the continuations — so
    `p.then(f).then(g)` converges with `let r = await p; let s = await f(r); g(s)`. This is why
    chains reduce recursively (evaluating the receiver triggers the inner `.then`). -/
theorem then_chain (p : α) (f : α → β) (g : β → γ) :
    thenP (thenP p f) g = thenP p (fun r => g (f r)) := rfl

/-- Local Promise state used for the rejection-channel slice. The value graph keeps this state
    behind a Promise boundary; the model here only proves which channel the local continuation
    selects. -/
inductive PromiseState (α : Type) where
  | fulfilled : α → PromiseState α
  | rejected : α → PromiseState α
deriving DecidableEq

open PromiseState

/-- One-argument `.then`: fulfilled values run the handler; rejected values propagate. -/
def thenLocal (p : PromiseState α) (onFulfilled : α → PromiseState α) : PromiseState α :=
  match p with
  | fulfilled value => onFulfilled value
  | rejected reason => rejected reason

/-- Two-argument `.then`: rejected values run the rejection handler. -/
def thenLocalRejected
    (p : PromiseState α)
    (onFulfilled : α → PromiseState α)
    (onRejected : α → PromiseState α) : PromiseState α :=
  match p with
  | fulfilled value => onFulfilled value
  | rejected reason => onRejected reason

/-- `.catch(h)` is the same local rejected-channel recovery as `.then(undefined, h)`: fulfilled
    values pass through, rejected values run the handler. -/
def catchLocal (p : PromiseState α) (onRejected : α → PromiseState α) : PromiseState α :=
  match p with
  | fulfilled value => fulfilled value
  | rejected reason => onRejected reason

/-- A handler that returns `Promise.resolve(v)` contributes `v` to the fulfilled channel rather
    than nesting a second Promise boundary. -/
theorem returned_resolve_flattens (p : α) :
    thenLocal (fulfilled p) (fun value => fulfilled value) = fulfilled p := rfl

/-- A handler that returns `Promise.reject(e)` preserves the rejected channel. -/
theorem returned_reject_preserves_channel (p reason : α) :
    thenLocal (fulfilled p) (fun _ => rejected reason) = rejected reason := rfl

/-- Local `catch` recovery on a rejected producer runs the rejection handler. -/
theorem catch_recovers_rejected (reason : α) (handler : α → PromiseState α) :
    catchLocal (rejected reason) handler = handler reason := rfl

/-- `p.catch(h)` and `p.then(undefined, h)` agree for rejected local producers. -/
theorem catch_eq_then_rejection_handler (reason : α) (onFulfilled handler : α → PromiseState α) :
    catchLocal (rejected reason) handler =
      thenLocalRejected (rejected reason) onFulfilled handler := rfl

end NosePromiseThen
