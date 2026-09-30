//! Membership duration settings use real sparse resolution and authority checks.

use crbk::*;
use serde_json::json;

fn snapshot(book: &Rulebook) -> Snapshot {
    Snapshot {
        community: "garden".into(),
        kind: SnapshotKind::Settings,
        revision: 1,
        policy_epoch: 1,
        issued: 0,
        content: book.resolve(&Values::new()).unwrap(),
    }
}

#[test]
fn membership_defaults_and_platform_force_use_the_real_resolver() {
    let mut book = Rulebook::default();
    assert!(MembershipSettings::from_snapshot(&snapshot(&book)).is_err());
    define_membership_settings(&mut book).unwrap();
    let initial = MembershipSettings::from_snapshot(&snapshot(&book)).unwrap();
    assert_eq!(
        initial,
        MembershipSettings {
            new_credential_days: 1,
            established_credential_days: 30,
            probation_days: 14,
        }
    );
    book.set_community(ESTABLISHED_CREDENTIAL_DAYS, Some(json!(7)))
        .unwrap();
    book.set_platform(
        ESTABLISHED_CREDENTIAL_DAYS,
        Some(PlatformValue {
            value: json!(3),
            force: true,
        }),
    )
    .unwrap();
    assert_eq!(
        MembershipSettings::from_snapshot(&snapshot(&book))
            .unwrap()
            .established_credential_days,
        3
    );
    assert!(
        book.resolve(&Values::from([(PROBATION_DAYS.into(), json!(1))]))
            .is_err()
    );
    for bad in [json!(null), json!(0), json!(31), json!(1.5), json!("1")] {
        assert!(book.set_community(NEW_CREDENTIAL_DAYS, Some(bad)).is_err());
    }
    let mut invalid = snapshot(&book);
    invalid.content.insert(PROBATION_DAYS.into(), json!(null));
    assert!(MembershipSettings::from_snapshot(&invalid).is_err());
}
