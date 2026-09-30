use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Error, Result, Values, identifier};

/// Scope of an already verified gate result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateLevel {
    /// Verified globally and bound to the subject by the caller.
    Global,
    /// Verified for this community.
    Community,
}

impl GateLevel {
    fn key(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Community => "community",
        }
    }
}

/// Catalogue key for a gate's boolean switch.
pub fn gate_key(level: GateLevel, gate: &str) -> String {
    format!("gate/{}/{gate}", level.key())
}

/// Catalogue key for a provider's boolean switch within a gate and level.
pub fn provider_key(level: GateLevel, gate: &str, provider: &str) -> String {
    format!("provider/{}/{gate}/{provider}", level.key())
}

/// Catalogue key for a typed action policy.
pub fn action_key(action: &str) -> String {
    format!("action/{action}")
}

/// Minimal membership state supplied by the membership facade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipState {
    /// Registration is still in progress.
    Pending,
    /// Community admission is current.
    Admitted,
    /// Previously admitted, but a requirement has lapsed.
    Lapsed,
    /// Membership has been released.
    Released,
}

/// Transient, authenticated subject context. Never stored by this crate.
pub struct Subject<'a> {
    /// Opaque subject identifier; global results must already be rebound by cgts.
    pub id: &'a str,
    /// Current membership state from cmbr.
    pub membership: MembershipState,
}

/// Verified proof metadata, never raw gate input or confidence scores.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateResult {
    /// Gate identifier.
    pub gate: String,
    /// Global or community scope.
    pub level: GateLevel,
    /// Subject to which verification bound the result.
    pub subject: String,
    /// Community binding for community-level results; absent for global results.
    pub community: Option<String>,
    /// Provider identifier.
    pub provider: String,
    /// Exclusive expiry in caller-supplied Unix seconds.
    pub valid_until: i64,
    /// Verified proof issuance time, required only for maximum-age policies.
    /// This is proof metadata, never a login time; not persisted here.
    pub proven_at: Option<i64>,
}

/// Adapter boundary for cgts until it exposes a shared result API.
pub trait GateResults {
    /// Return verified metadata. Verification and subject binding belong to cgts.
    fn gate_results(&self) -> &[GateResult];
}

impl GateResults for [GateResult] {
    fn gate_results(&self) -> &[GateResult] {
        self
    }
}

impl GateResults for Vec<GateResult> {
    fn gate_results(&self) -> &[GateResult] {
        self
    }
}

/// A required gate, optionally restricted to one provider.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    /// Gate identifier.
    pub gate: String,
    /// Required scope.
    pub level: GateLevel,
    /// Optional provider restriction.
    pub provider: Option<String>,
}

/// At least k distinct gate/level pairs from a set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Threshold {
    /// Required number, from one through the number of gates.
    pub k: usize,
    /// Distinct gate/level pairs; duplicate providers cannot inflate the count.
    pub of: Vec<Requirement>,
}

/// Small conjunctive policy: all_of AND any_of AND k_of_n AND membership.
/// Empty all_of/any_of groups impose no requirement; an empty policy permits.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionPolicy {
    /// Every listed gate must pass.
    pub all_of: Vec<Requirement>,
    /// At least one listed gate must pass; empty disables this group.
    pub any_of: Vec<Requirement>,
    /// Optional threshold group.
    pub k_of_n: Option<Threshold>,
    /// Inclusive maximum age in seconds; future proofs always fail.
    pub maximum_proof_age: Option<u64>,
    /// Allowed states; empty imposes no membership requirement (e.g. enrolment).
    pub membership: BTreeSet<MembershipState>,
}

impl ActionPolicy {
    /// Reject ambiguous duplicate gates, invalid identifiers and impossible k.
    pub fn validate(&self) -> Result<()> {
        let mut groups = vec![&self.all_of, &self.any_of];
        if let Some(threshold) = &self.k_of_n {
            if threshold.k == 0 || threshold.k > threshold.of.len() {
                return Err(Error::Invalid("invalid threshold"));
            }
            groups.push(&threshold.of);
        }
        for group in groups {
            let mut unique = BTreeSet::new();
            for requirement in group {
                identifier(&requirement.gate)?;
                if let Some(provider) = &requirement.provider {
                    identifier(provider)?;
                }
                if !unique.insert((&requirement.gate, requirement.level)) {
                    return Err(Error::Invalid("duplicate gate in policy group"));
                }
            }
        }
        Ok(())
    }
}

/// Why a gate requirement did not pass. Contains no supplied proof data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateFailure {
    /// Gate is disabled or has no true switch.
    Disabled,
    /// A specifically required provider is disabled.
    ProviderDisabled,
    /// No matching current proof from an enabled provider passed age checks.
    ProofRequired,
}

/// A failing gate alternative, suitable for a headless lobby.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingGate {
    /// Requirement rather than proof or subject details.
    pub requirement: Requirement,
    /// Reason this alternative cannot currently count.
    pub reason: GateFailure,
}

/// An unsatisfied conjunct, retaining alternatives and the remaining count.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Missing {
    /// No enabled action policy exists; default deny.
    ActionUnavailable,
    /// Subject is not in an allowed membership state.
    Membership {
        /// Acceptable states, without exposing the supplied subject.
        allowed: BTreeSet<MembershipState>,
    },
    /// Complete this many additional alternatives to satisfy the group.
    Gates {
        /// Required number of additional passes.
        needed: usize,
        /// Remaining choices. Disabled choices should not be offered in the UI.
        alternatives: Vec<MissingGate>,
    },
}

/// A pure verdict containing no identity, timestamps or raw results.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// True exactly when nothing is missing.
    pub allowed: bool,
    /// Unsatisfied conjuncts; an empty list means yes.
    pub missing: Vec<Missing>,
}

/// Flat, unsigned community settings suitable for serialization and csgn.
/// Member latitude is deliberately absent from this shared publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotKind {
    /// Resolved rulebook settings.
    Settings,
}

/// Flat unsigned publication; csgn supplies its key ID and signature envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Community namespace.
    pub community: String,
    /// Payload discriminator for the signing facade.
    pub kind: SnapshotKind,
    /// Immutable rulebook revision.
    pub revision: u64,
    /// Policy epoch selected by the caller.
    pub policy_epoch: u64,
    /// Snapshot issuance time, not a member event time.
    pub issued: i64,
    /// Flat key-to-whole-JSON-value content.
    pub content: Values,
}

impl Snapshot {
    /// Evaluate only verified metadata; performs no I/O and records nothing.
    pub fn may(
        &self,
        subject: Subject<'_>,
        action: &str,
        results: &(impl GateResults + ?Sized),
        now: i64,
    ) -> Result<Decision> {
        identifier(action)?;
        if now < 0 {
            return Err(Error::Invalid("negative time"));
        }
        if self.issued < 0 || self.issued > now {
            return Err(Error::Invalid("snapshot not yet issued"));
        }
        if subject.id.is_empty() || self.community.is_empty() {
            return Err(Error::Invalid("missing decision scope"));
        }
        let Some(value) = self
            .content
            .get(&action_key(action))
            .filter(|v| !v.is_null())
        else {
            return Ok(Decision {
                allowed: false,
                missing: vec![Missing::ActionUnavailable],
            });
        };
        let policy: ActionPolicy =
            serde_json::from_value(value.clone()).map_err(|_| Error::Invalid("action policy"))?;
        policy.validate()?;
        let mut missing = Vec::new();
        if !policy.membership.is_empty() && !policy.membership.contains(&subject.membership) {
            missing.push(Missing::Membership {
                allowed: policy.membership.clone(),
            });
        }
        let mut groups = vec![
            (&policy.all_of, policy.all_of.len()),
            (&policy.any_of, usize::from(!policy.any_of.is_empty())),
        ];
        if let Some(threshold) = &policy.k_of_n {
            groups.push((&threshold.of, threshold.k));
        }
        for (requirements, needed) in groups {
            let alternatives: Vec<_> = requirements
                .iter()
                .filter_map(|r| {
                    self.failure(
                        r,
                        &subject,
                        results.gate_results(),
                        policy.maximum_proof_age,
                        now,
                    )
                    .map(|reason| MissingGate {
                        requirement: r.clone(),
                        reason,
                    })
                })
                .collect();
            let passed = requirements.len() - alternatives.len();
            if passed < needed {
                missing.push(Missing::Gates {
                    needed: needed - passed,
                    alternatives,
                });
            }
        }
        Ok(Decision {
            allowed: missing.is_empty(),
            missing,
        })
    }

    fn enabled(&self, key: &str) -> bool {
        self.content.get(key).and_then(|v| v.as_bool()) == Some(true)
    }

    fn failure(
        &self,
        requirement: &Requirement,
        subject: &Subject<'_>,
        results: &[GateResult],
        age: Option<u64>,
        now: i64,
    ) -> Option<GateFailure> {
        if !self.enabled(&gate_key(requirement.level, &requirement.gate)) {
            return Some(GateFailure::Disabled);
        }
        if requirement
            .provider
            .as_ref()
            .is_some_and(|p| !self.enabled(&provider_key(requirement.level, &requirement.gate, p)))
        {
            return Some(GateFailure::ProviderDisabled);
        }
        let passed = results.iter().any(|proof| {
            proof.subject == subject.id
                && proof.gate == requirement.gate
                && proof.level == requirement.level
                && match proof.level {
                    GateLevel::Global => proof.community.is_none(),
                    GateLevel::Community => {
                        proof.community.as_deref() == Some(self.community.as_str())
                    }
                }
                && requirement
                    .provider
                    .as_ref()
                    .is_none_or(|p| *p == proof.provider)
                && identifier(&proof.provider).is_ok()
                && self.enabled(&provider_key(proof.level, &proof.gate, &proof.provider))
                && proof.valid_until > now
                && proof
                    .proven_at
                    .is_none_or(|t| t >= 0 && t <= now && t < proof.valid_until)
                && age.is_none_or(|max| proof.proven_at.is_some_and(|t| (now - t) as u64 <= max))
        });
        if passed {
            None
        } else {
            Some(GateFailure::ProofRequired)
        }
    }
}
