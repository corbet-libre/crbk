//! Pure gate decisions, proof boundaries and missing alternatives.

use crate::common::*;
use crbk::*;
use serde_json::json;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn all_any_and_threshold_keep_missing_alternatives() {
    let policy = ActionPolicy {
        all_of: vec![requirement("voucher")],
        any_of: vec![requirement("phone"), requirement("payment")],
        k_of_n: Some(Threshold {
            k: 2,
            of: vec![
                requirement("voucher"),
                requirement("phone"),
                requirement("payment"),
            ],
        }),
        maximum_proof_age: Some(10),
        membership: [MembershipState::Admitted].into(),
    };
    let s = snapshot(decision_book(policy));
    let proofs = vec![proof("voucher")];
    let decision = s.may(subject(), "enter", &proofs, 100).unwrap();
    assert!(!decision.allowed);
    assert_eq!(decision.missing.len(), 2);
    assert!(
        matches!(&decision.missing[0], Missing::Gates { needed: 1, alternatives } if alternatives.len() == 2)
    );
    let proofs = vec![proof("voucher"), proof("phone")];
    assert!(s.may(subject(), "enter", &proofs, 100).unwrap().allowed);
    assert!(
        !s.may(
            Subject {
                id: "member",
                membership: MembershipState::Pending
            },
            "enter",
            &proofs,
            100
        )
        .unwrap()
        .allowed
    );
    assert!(!s.may(subject(), "unknown", &proofs, 100).unwrap().allowed);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn combinations_count_distinct_gates_not_duplicate_proofs() {
    for k in 1..=3 {
        let policy = ActionPolicy {
            k_of_n: Some(Threshold {
                k,
                of: vec![
                    requirement("voucher"),
                    requirement("phone"),
                    requirement("payment"),
                ],
            }),
            ..Default::default()
        };
        let s = snapshot(decision_book(policy));
        for mask in 0_u32..8 {
            let proofs: Vec<_> = ["voucher", "phone", "payment"]
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .flat_map(|(_, g)| [proof(g), proof(g)])
                .collect();
            assert_eq!(
                s.may(subject(), "enter", &proofs, 100).unwrap().allowed,
                mask.count_ones() as usize >= k
            );
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn proof_subject_scope_provider_expiry_and_age_are_checked() {
    let policy = ActionPolicy {
        all_of: vec![requirement("voucher")],
        maximum_proof_age: Some(10),
        ..Default::default()
    };
    let s = snapshot(decision_book(policy));
    let original = proof("voucher");
    assert!(
        s.may(subject(), "enter", &vec![original.clone()], 100)
            .unwrap()
            .allowed
    );
    let mut variants = Vec::new();
    let mut p = original.clone();
    p.subject = "other".into();
    variants.push(p);
    let mut p = original.clone();
    p.community = Some("other".into());
    variants.push(p);
    let mut p = original.clone();
    p.community = None;
    variants.push(p);
    let mut p = original.clone();
    p.level = GateLevel::Global;
    variants.push(p);
    let mut p = original.clone();
    p.provider = "unknown".into();
    variants.push(p);
    let mut p = original.clone();
    p.valid_until = 100;
    variants.push(p);
    let mut p = original.clone();
    p.proven_at = Some(89);
    variants.push(p);
    let mut p = original.clone();
    p.proven_at = Some(101);
    variants.push(p);
    let mut p = original.clone();
    p.proven_at = Some(-1);
    variants.push(p);
    let mut p = original.clone();
    p.proven_at = None;
    variants.push(p);
    for p in variants {
        assert!(
            !s.may(subject(), "enter", &vec![p.clone()], 100)
                .unwrap()
                .allowed
        );
        assert!(
            s.may(subject(), "enter", &vec![p, original.clone()], 100)
                .unwrap()
                .allowed
        );
    }
    assert!(s.may(subject(), "enter", &vec![original], -1).is_err());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn absent_age_metadata_allowed_without_age_constraint_and_global_binding() {
    let mut requirement = requirement("phone");
    requirement.level = GateLevel::Global;
    let policy = ActionPolicy {
        all_of: vec![requirement],
        ..Default::default()
    };
    let mut s = snapshot(decision_book(policy));
    s.content
        .insert(gate_key(GateLevel::Global, "phone"), json!(true));
    s.content.insert(
        provider_key(GateLevel::Global, "phone", "local"),
        json!(true),
    );
    let mut p = proof("phone");
    p.level = GateLevel::Global;
    p.community = None;
    p.proven_at = None;
    assert!(
        s.may(subject(), "enter", &vec![p.clone()], 100)
            .unwrap()
            .allowed
    );
    p.community = Some("alpha".into());
    assert!(!s.may(subject(), "enter", &vec![p], 100).unwrap().allowed);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn disabled_gate_provider_action_and_platform_force_fail_closed() {
    let mut r = requirement("voucher");
    r.provider = Some("local".into());
    let policy = ActionPolicy {
        all_of: vec![r],
        ..Default::default()
    };
    let base = decision_book(policy);
    for key in [
        gate_key(GateLevel::Community, "voucher"),
        provider_key(GateLevel::Community, "voucher", "local"),
    ] {
        for value in [json!(false), json!(null)] {
            let mut b = base.clone();
            b.set_community(&key, Some(json!(true))).unwrap();
            b.set_platform(&key, platform(value, true)).unwrap();
            assert!(
                !snapshot(b)
                    .may(subject(), "enter", &vec![proof("voucher")], 100)
                    .unwrap()
                    .allowed
            );
        }
    }
    let mut b = base;
    b.set_community(&action_key("enter"), Some(json!(null)))
        .unwrap();
    assert_eq!(
        snapshot(b)
            .may(subject(), "enter", &vec![], 100)
            .unwrap()
            .missing,
        vec![Missing::ActionUnavailable]
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn policies_reject_invalid_thresholds_and_duplicate_provider_variants() {
    for k in [0, 3] {
        assert!(
            ActionPolicy {
                k_of_n: Some(Threshold {
                    k,
                    of: vec![requirement("voucher"), requirement("phone")]
                }),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    let one = requirement("voucher");
    let mut two = one.clone();
    two.provider = Some("different".into());
    assert!(
        ActionPolicy {
            any_of: vec![one, two],
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    let s = snapshot(decision_book(ActionPolicy::default()));
    assert!(s.may(subject(), "enter", &vec![], 100).unwrap().allowed);
    let mut bad = s;
    bad.content
        .insert(action_key("enter"), json!({"all_of": "secret-input"}));
    let error = bad.may(subject(), "enter", &vec![], 100).unwrap_err();
    assert!(!format!("{error:?}").contains("secret-input"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn snapshots_and_verdicts_are_serializable_without_member_traces() {
    let b = decision_book(ActionPolicy {
        all_of: vec![requirement("voucher")],
        ..Default::default()
    });
    let s = snapshot(b);
    let encoded = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<Snapshot>(&encoded).unwrap(), s);
    let d = s
        .may(
            Subject {
                id: "private-subject",
                membership: MembershipState::Admitted,
            },
            "enter",
            &vec![proof("voucher")],
            100,
        )
        .unwrap();
    let encoded = serde_json::to_string(&d).unwrap();
    assert!(!encoded.contains("private-subject"));
    assert!(!encoded.contains("proven_at"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn future_snapshots_and_empty_subjects_cannot_authorize() {
    let mut s = snapshot(decision_book(ActionPolicy::default()));
    assert!(
        s.may(
            Subject {
                id: "",
                membership: MembershipState::Admitted
            },
            "enter",
            &vec![],
            100
        )
        .is_err()
    );
    s.issued = 101;
    assert!(s.may(subject(), "enter", &vec![], 100).is_err());
    assert!(s.may(subject(), "enter", &vec![], 101).unwrap().allowed);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn global_proof_bindings_and_slice_adapter_preserve_scope() {
    let mut requirement = requirement("voucher");
    requirement.level = GateLevel::Global;
    requirement.provider = Some("local".into());
    let mut snapshot = snapshot(decision_book(ActionPolicy {
        all_of: vec![requirement],
        ..Default::default()
    }));
    snapshot
        .content
        .insert(gate_key(GateLevel::Global, "voucher"), json!(true));
    snapshot.content.insert(
        provider_key(GateLevel::Global, "voucher", "local"),
        json!(true),
    );
    let mut proof = proof("voucher");
    proof.level = GateLevel::Global;
    proof.community = None;
    let proofs = [proof.clone()];
    assert!(
        snapshot
            .may(subject(), "enter", proofs.as_slice(), 100)
            .unwrap()
            .allowed
    );
    proof.community = Some("alpha".into());
    assert!(
        !snapshot
            .may(subject(), "enter", &[proof.clone()][..], 100)
            .unwrap()
            .allowed
    );
    proof.community = None;
    proof.provider = "other".into();
    assert!(
        !snapshot
            .may(subject(), "enter", &[proof][..], 100)
            .unwrap()
            .allowed
    );
    snapshot.issued = -1;
    assert!(
        snapshot
            .may(subject(), "enter", proofs.as_slice(), 100)
            .is_err()
    );
    snapshot.issued = 100;
    snapshot.community.clear();
    assert!(
        snapshot
            .may(subject(), "enter", proofs.as_slice(), 100)
            .is_err()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn malformed_unrestricted_provider_cannot_satisfy_an_enabled_gate() {
    let snapshot = snapshot(decision_book(ActionPolicy {
        all_of: vec![requirement("voucher")],
        ..Default::default()
    }));
    let mut proof = proof("voucher");
    proof.provider = "invalid provider".into();
    let decision = snapshot.may(subject(), "enter", &[proof][..], 100).unwrap();
    assert!(!decision.allowed);
}
