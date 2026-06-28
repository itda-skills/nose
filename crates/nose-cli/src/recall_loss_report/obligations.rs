pub(super) fn rejection_obligation(
    reason: &'static str,
    missing_evidence: &[&'static str],
) -> (&'static str, &'static str) {
    let first_missing = missing_evidence
        .first()
        .copied()
        .unwrap_or("unknown-obligation-proof");
    match reason {
        "hof-demand-effect-proof-missing" => (
            "callback-demand-effect",
            hof_demand_effect_obligation_subreason(missing_evidence),
        ),
        "mutation-effect-boundary" => ("receiver-mutation", "effect-preserving-contract-missing"),
        "unsupported-runtime-boundary" => runtime_boundary_obligation(missing_evidence),
        "receiver-domain-proof-missing" => (
            "ambiguous-selector-boundary",
            "receiver-domain-proof-missing",
        ),
        "library-api-occurrence-proof-missing" => (
            "ambiguous-selector-boundary",
            "library-api-occurrence-evidence-missing",
        ),
        "source-surface-proof-missing" => match first_missing {
            "rust-macro-expansion-contract" => (
                "source-protocol-boundary",
                "rust-macro-expansion-contract-missing",
            ),
            _ => (
                "source-protocol-boundary",
                "source-surface-contract-missing",
            ),
        },
        "import-symbol-callee-identity-proof-missing" => {
            ("ambiguous-selector-boundary", first_missing)
        }
        "value-fingerprint-too-small" => (
            "non-degenerate-fingerprint-floor",
            "non-degenerate-value-fingerprint",
        ),
        "unattributed-strict-exact-unsafe" => {
            ("unattributed-boundary", "strict-exact-safe-tree-missing")
        }
        _ => ("unattributed-boundary", first_missing),
    }
}

type Obligation = (&'static str, &'static str);

struct RuntimeBoundaryRule {
    evidence: &'static str,
    obligation: Obligation,
}

const DEFAULT_RUNTIME_BOUNDARY_OBLIGATION: Obligation = (
    "scheduling-boundary",
    "runtime-protocol-boundary-contract-missing",
);

const RUNTIME_BOUNDARY_OBLIGATIONS: &[RuntimeBoundaryRule] = &[
    RuntimeBoundaryRule {
        evidence: "promise-await-scheduling-contract",
        obligation: (
            "scheduling-boundary",
            "promise-await-scheduling-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-async-function-scheduling-contract",
        obligation: (
            "scheduling-boundary",
            "promise-async-function-scheduling-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "future-async-block-scheduling-contract",
        obligation: (
            "scheduling-boundary",
            "future-async-block-scheduling-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-executor-callback-effect-contract",
        obligation: (
            "executor-callback",
            "promise-executor-callback-effect-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-then-promise-like-receiver-proof",
        obligation: (
            "ambiguous-selector-boundary",
            "promise-then-promise-like-receiver-proof-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-then-fulfillment-continuation-contract",
        obligation: (
            "success-error-result-channel",
            "promise-then-fulfillment-continuation-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-then-rejection-continuation-contract",
        obligation: (
            "rejection-channel",
            "promise-then-rejection-continuation-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-then-callback-demand-effect-contract",
        obligation: (
            "callback-demand-effect",
            "promise-then-callback-demand-effect-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-catch-rejection-continuation-contract",
        obligation: (
            "rejection-channel",
            "promise-catch-rejection-continuation-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-finally-settlement-continuation-contract",
        obligation: (
            "rejection-channel",
            "promise-finally-settlement-continuation-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-reject-rejected-value-channel-contract",
        obligation: (
            "rejection-channel",
            "promise-reject-rejected-value-channel-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-catch-callback-demand-effect-contract",
        obligation: (
            "callback-demand-effect",
            "promise-catch-callback-demand-effect-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-finally-callback-demand-effect-contract",
        obligation: (
            "callback-demand-effect",
            "promise-finally-callback-demand-effect-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-rejection-continuation-contract",
        obligation: (
            "rejection-channel",
            "promise-rejection-continuation-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-rejection-channel-contract",
        obligation: (
            "rejection-channel",
            "promise-rejection-channel-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-aggregate-result-channel-contract",
        obligation: (
            "success-error-result-channel",
            "promise-aggregate-result-channel-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-factory-settled-value-contract",
        obligation: (
            "success-error-result-channel",
            "promise-factory-settled-value-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-non-construct-call-boundary-contract",
        obligation: (
            "scheduling-boundary",
            "promise-non-construct-call-boundary-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "promise-like-receiver-proof",
        obligation: (
            "ambiguous-selector-boundary",
            "promise-like-receiver-proof-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "channel-protocol-contract",
        obligation: ("channel-boundary", "channel-protocol-contract-missing"),
    },
    RuntimeBoundaryRule {
        evidence: "exception-channel-contract",
        obligation: ("exception-channel", "exception-channel-contract-missing"),
    },
    RuntimeBoundaryRule {
        evidence: "generator-yield-protocol-contract",
        obligation: (
            "lifecycle-materialization-boundary",
            "generator-yield-protocol-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "concurrency-scheduling-contract",
        obligation: (
            "scheduling-boundary",
            "concurrency-scheduling-contract-missing",
        ),
    },
    RuntimeBoundaryRule {
        evidence: "runtime-call-shape-contract",
        obligation: (
            "source-protocol-boundary",
            "runtime-call-shape-contract-missing",
        ),
    },
];

fn runtime_boundary_obligation(missing_evidence: &[&'static str]) -> Obligation {
    RUNTIME_BOUNDARY_OBLIGATIONS
        .iter()
        .find(|rule| missing_evidence.contains(&rule.evidence))
        .map(|rule| rule.obligation)
        .unwrap_or(DEFAULT_RUNTIME_BOUNDARY_OBLIGATION)
}

fn hof_demand_effect_obligation_subreason(missing_evidence: &[&'static str]) -> &'static str {
    if missing_evidence.contains(&"hof-callback-runtime-boundary-proof") {
        return "callback-runtime-boundary-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-assignment-effect-proof") {
        return "callback-assignment-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-direct-function-call-effect-proof") {
        return "callback-direct-function-call-effect-contract-missing";
    }
    if missing_evidence.contains(&"hof-callback-direct-method-call-effect-proof") {
        return "callback-direct-method-call-effect-contract-missing";
    }
    if missing_evidence.contains(&"hof-callback-imported-function-call-effect-proof") {
        return "callback-imported-function-call-effect-contract-missing";
    }
    if missing_evidence.contains(&"hof-callback-imported-member-call-effect-proof") {
        return "callback-imported-member-call-effect-contract-missing";
    }
    if missing_evidence.contains(&"hof-callback-dynamic-dispatch-call-effect-proof") {
        return "callback-dynamic-dispatch-call-effect-contract-missing";
    }
    if missing_evidence.contains(&"hof-callback-builtin-call-effect-proof") {
        return "callback-builtin-call-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-rust-macro-call-effect-proof") {
        return "callback-rust-macro-call-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-scoped-path-call-effect-proof") {
        return "callback-scoped-path-call-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-imported-binding-call-effect-proof") {
        return "callback-imported-binding-call-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-qualified-global-call-effect-proof") {
        return "callback-qualified-global-call-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-unshadowed-global-call-effect-proof") {
        return "callback-unshadowed-global-call-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-member-call-effect-proof") {
        return "callback-member-call-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-local-or-parameter-call-effect-proof") {
        return "callback-local-or-parameter-call-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-rejected-call-target-effect-proof") {
        return "callback-rejected-call-target-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-unknown-call-effect-proof") {
        return "callback-unknown-call-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-call-effect-proof") {
        return "callback-call-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-effect-proof") {
        return "callback-effect-proof-missing";
    }
    if missing_evidence.contains(&"hof-callback-identity-proof")
        || missing_evidence.contains(&"hof-callback-arity-shape-proof")
    {
        return "callback-identity-or-shape-proof-missing";
    }
    if missing_evidence.contains(&"hof-reduce-callback-demand-effect-profile") {
        return "reduction-callback-demand-effect-profile-missing";
    }
    if missing_evidence.contains(&"hof-filter-map-callback-demand-effect-profile") {
        return "optional-callback-demand-effect-profile-missing";
    }
    if missing_evidence.contains(&"hof-flat-map-callback-demand-effect-profile") {
        return "flattening-callback-demand-effect-profile-missing";
    }
    if missing_evidence.contains(&"hof-filter-callback-demand-effect-profile")
        || missing_evidence.contains(&"hof-reject-callback-demand-effect-profile")
    {
        return "predicate-callback-demand-effect-profile-missing";
    }
    if missing_evidence.contains(&"hof-map-callback-demand-effect-profile") {
        return "mapping-callback-demand-effect-profile-missing";
    }
    if missing_evidence.contains(&"hof-source-or-library-api-occurrence-proof") {
        return "hof-source-or-library-api-occurrence-proof-missing";
    }
    "hof-demand-effect-profile-missing"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promise_then_receiver_obligation_is_primary_for_selector_only_then() {
        let labels = [
            "lowered-runtime-boundary-contract",
            "promise-then-promise-like-receiver-proof",
            "promise-then-fulfillment-continuation-contract",
            "promise-then-rejection-continuation-contract",
            "promise-then-callback-demand-effect-contract",
        ];

        assert_eq!(
            runtime_boundary_obligation(&labels),
            (
                "ambiguous-selector-boundary",
                "promise-then-promise-like-receiver-proof-missing",
            )
        );
    }

    #[test]
    fn promise_then_continuation_and_callback_labels_have_standalone_obligations() {
        assert_eq!(
            runtime_boundary_obligation(&["promise-then-fulfillment-continuation-contract"]),
            (
                "success-error-result-channel",
                "promise-then-fulfillment-continuation-contract-missing",
            )
        );
        assert_eq!(
            runtime_boundary_obligation(&["promise-then-rejection-continuation-contract"]),
            (
                "rejection-channel",
                "promise-then-rejection-continuation-contract-missing",
            )
        );
        assert_eq!(
            runtime_boundary_obligation(&["promise-then-callback-demand-effect-contract"]),
            (
                "callback-demand-effect",
                "promise-then-callback-demand-effect-contract-missing",
            )
        );
    }
}
