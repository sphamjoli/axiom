## Summary

<!-- 1-3 bullet points describing what this PR does and why -->

## Type

- [ ] Bug fix
- [ ] Feature
- [ ] Chore (refactor, deps, CI, docs)
- [ ] Breaking change (certificate format or formal semantics)

## Component

- [ ] Engine (Rust)
- [ ] Checker (`lean/`)
- [ ] Docs
- [ ] CI / infra

## Related issues

<!-- Closes #123, Relates to #456 -->

## Validation

<!-- Commands run and their results -->

## Checklist

- [ ] Title follows `type(scope): description` (e.g. `feat(cert): emit factorisation certificates`)
- [ ] `make check` passes locally
- [ ] Every new claim kind the engine emits has a Lean checker and an adversarial certificate that the checker rejects
- [ ] No `sorry`, `admit`, new `axiom`, or `native_decide` in `lean/`
- [ ] Trusted computing base changes (if any) are stated and labelled `tcb`
- [ ] No secrets, local planning notes, or build artifacts committed
