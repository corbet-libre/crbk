//! The platform tunable catalogue: every number the product lets Root or a
//! community change, with its type, default, bounds and lowest override layer,
//! plus the human text a generic settings screen needs. Behaviour stays fixed
//! in code; only these values are settings.

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value, json};

use crate::{Bounds, Layer, PlatformValue, Result, Rulebook, Setting, SettingKind, SettingType};

/// Fixed-point scale of the standing parameters (the proof circuit's Q16).
pub const Q: u64 = 1 << 16;

/// Voucher allowance slope per messaging-cap unit for ordinary members.
pub const VOUCHER_SLOPE_MEMBER: &str = "voucher.slope.member";
/// Voucher allowance slope for admins.
pub const VOUCHER_SLOPE_ADMIN: &str = "voucher.slope.admin";
/// Voucher allowance slope for roots.
pub const VOUCHER_SLOPE_ROOT: &str = "voucher.slope.root";
/// Voucher allowance lower bound for members.
pub const VOUCHER_MIN_MEMBER: &str = "voucher.min.member";
/// Voucher allowance upper bound for members.
pub const VOUCHER_MAX_MEMBER: &str = "voucher.max.member";
/// Voucher allowance lower bound for admins.
pub const VOUCHER_MIN_ADMIN: &str = "voucher.min.admin";
/// Voucher allowance upper bound for admins.
pub const VOUCHER_MAX_ADMIN: &str = "voucher.max.admin";
/// Voucher allowance lower bound for roots.
pub const VOUCHER_MIN_ROOT: &str = "voucher.min.root";
/// Voucher allowance upper bound for roots.
pub const VOUCHER_MAX_ROOT: &str = "voucher.max.root";
/// Whether a successful vouchee's own success raises the sponsor's limit.
pub const VOUCHER_REWARD_ENABLED: &str = "voucher.reward.enabled";
/// Permanent reward bonus cap per sponsor.
pub const VOUCHER_REWARD_CAP: &str = "voucher.reward.cap";
/// Sign-off default for single-use links.
pub const VOUCHER_SIGNOFF_SINGLE: &str = "voucher.signoff.single_use";
/// Sign-off default for multi-use links.
pub const VOUCHER_SIGNOFF_MULTI: &str = "voucher.signoff.multi_use";
/// Whether admitted members show who vouched (one-way, see the help text).
pub const VOUCHER_SPONSOR_VISIBLE: &str = "voucher.sponsor_visible";
/// Days since joining before a vouchee can prove success.
pub const SUCCESS_MIN_DAYS: &str = "voucher.success.min_days";
/// Punishments received a successful vouchee may have at most.
pub const SUCCESS_MAX_PUNISHMENTS: &str = "voucher.success.max_punishments";
/// Answered conversations with distinct established members required.
pub const SUCCESS_MIN_CONVERSATIONS: &str = "voucher.success.min_conversations";
/// Days in the community that make a conversation partner established.
pub const SUCCESS_PARTNER_DAYS: &str = "voucher.success.partner_min_days";
/// Optional block: minimum acceptance rate of the vouchee's own waves.
pub const SUCCESS_MIN_ACCEPTANCE: &str = "voucher.success.min_acceptance_rate";
/// Optional block: distinct active weeks.
pub const SUCCESS_MIN_WEEKS: &str = "voucher.success.min_active_weeks";
/// Optional block: standing threshold (messaging cap).
pub const SUCCESS_MIN_STANDING: &str = "voucher.success.min_standing";
/// Optional block: punishments given at most.
pub const SUCCESS_MAX_GIVEN: &str = "voucher.success.max_punishments_given";
/// Optional block: the vouchee passed later gates.
pub const SUCCESS_REQUIRE_GATES: &str = "voucher.success.require_later_gates";
/// Optional block: the vouchee has vouched successfully themselves.
pub const SUCCESS_REQUIRE_VOUCHED: &str = "voucher.success.require_vouched";
/// New communities one person may start per rolling seven days.
pub const CREATE_PER_WEEK: &str = "community.create.per_week";
/// New communities one person may start per rolling 365 days.
pub const CREATE_PER_YEAR: &str = "community.create.per_year";
/// Shortest community address.
pub const ADDRESS_MIN: &str = "community.address.min_length";
/// Longest community address.
pub const ADDRESS_MAX: &str = "community.address.max_length";
/// Root-maintained reserved community addresses.
pub const ADDRESS_RESERVED: &str = "community.address.reserved";
const WAVES_WAITING_PERIOD: &str = "waves.waiting_period_days";

/// Prefix of the standing parameters; the suffix is the czkp field name.
pub const STANDING_PREFIX: &str = "standing.";

/// Human text and presentation hints for one setting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingMeta {
    /// Screen section.
    pub group: String,
    /// Short label.
    pub label: String,
    /// One or two sentences of help.
    pub help: String,
    /// Display unit, such as `days`; fixed-point values say `q16`.
    pub unit: Option<String>,
    /// Turning this setting off is permanent for earlier cases; the screen
    /// asks for confirmation before it is switched the one-way direction.
    pub one_way: bool,
}

/// One catalogue entry: key, definition and presentation text.
#[derive(Clone, Debug, PartialEq)]
pub struct CatalogEntry {
    /// Dotted setting key.
    pub key: String,
    /// Definition, including type, default, bounds and lowest layer.
    pub setting: Setting,
    /// Presentation text.
    pub meta: SettingMeta,
}

/// What a generic settings screen needs for one setting, resolved for a community.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingView {
    /// Dotted setting key.
    pub key: String,
    /// Presentation text.
    pub meta: SettingMeta,
    /// Value type.
    pub value_type: SettingType,
    /// Whether explicit null is a legal value.
    pub nullable: bool,
    /// Inclusive minimum.
    pub min: Option<Number>,
    /// Inclusive maximum.
    pub max: Option<Number>,
    /// Deepest layer that may override.
    pub lowest_layer: Layer,
    /// Live technical or pinned template setting.
    pub kind: SettingKind,
    /// Catalogue default.
    pub default: Value,
    /// Platform row, when set.
    pub platform: Option<PlatformValue>,
    /// Community deviation, when set (explicit null is a value).
    pub community: Option<Value>,
    /// The resolved value everyone sees (member latitude excluded).
    pub effective: Value,
}

fn meta(group: &str, label: &str, help: &str, unit: Option<&str>, one_way: bool) -> SettingMeta {
    SettingMeta {
        group: group.into(),
        label: label.into(),
        help: help.into(),
        unit: unit.map(Into::into),
        one_way,
    }
}

fn num(n: f64) -> Number {
    Number::from_f64(n).expect("catalogue bounds are finite")
}

struct Row {
    key: String,
    value_type: SettingType,
    nullable: bool,
    default: Value,
    min: Option<Number>,
    max: Option<Number>,
    layer: Layer,
}

fn int(key: &str, default: i64, min: i64, max: i64, layer: Layer) -> Row {
    Row {
        key: key.into(),
        value_type: SettingType::Integer,
        nullable: false,
        default: default.into(),
        min: Some(min.into()),
        max: Some(max.into()),
        layer,
    }
}

fn float(key: &str, default: f64, min: f64, max: f64, layer: Layer) -> Row {
    Row {
        key: key.into(),
        value_type: SettingType::Number,
        nullable: false,
        default: json!(default),
        min: Some(num(min)),
        max: Some(num(max)),
        layer,
    }
}

fn flag(key: &str, default: bool, layer: Layer) -> Row {
    Row {
        key: key.into(),
        value_type: SettingType::Boolean,
        nullable: false,
        default: default.into(),
        min: None,
        max: None,
        layer,
    }
}

fn optional_int(key: &str, min: i64, max: i64) -> Row {
    Row {
        nullable: true,
        default: Value::Null,
        ..int(key, 0, min, max, Layer::Community)
    }
}

fn entry(row: Row, meta: SettingMeta) -> CatalogEntry {
    CatalogEntry {
        key: row.key,
        setting: Setting {
            value_type: row.value_type,
            nullable: row.nullable,
            default: row.default,
            bounds: Bounds {
                min: row.min,
                max: row.max,
            },
            lowest_layer: row.layer,
            kind: SettingKind::Technical,
        },
        meta,
    }
}

fn voucher_rows() -> Vec<CatalogEntry> {
    use Layer::{Community, Platform};
    let g = "Vouchers";
    let role =
        |name: &str| format!("Root sets the {name} allowance; communities cannot change it.");
    let mut out = Vec::new();
    for (slope, min, max, key_slope, key_min, key_max, role_name) in [
        (
            0.1,
            1,
            3,
            VOUCHER_SLOPE_MEMBER,
            VOUCHER_MIN_MEMBER,
            VOUCHER_MAX_MEMBER,
            "member",
        ),
        (
            2.0,
            50,
            100,
            VOUCHER_SLOPE_ADMIN,
            VOUCHER_MIN_ADMIN,
            VOUCHER_MAX_ADMIN,
            "admin",
        ),
        (
            25.0,
            500,
            1000,
            VOUCHER_SLOPE_ROOT,
            VOUCHER_MIN_ROOT,
            VOUCHER_MAX_ROOT,
            "root",
        ),
    ] {
        out.push(entry(
            float(key_slope, slope, 0.0, 1000.0, Platform),
            meta(
                g,
                &format!("Allowance slope, {role_name}"),
                &format!(
                    "Vouchers per unit of messaging cap before the limits apply. {}",
                    role(role_name)
                ),
                None,
                false,
            ),
        ));
        out.push(entry(
            int(key_min, min, 0, 100_000, Platform),
            meta(
                g,
                &format!("Allowance minimum, {role_name}"),
                &format!(
                    "Lowest voucher limit for a {role_name}. {}",
                    role(role_name)
                ),
                Some("vouchers"),
                false,
            ),
        ));
        out.push(entry(
            int(key_max, max, 0, 100_000, Platform),
            meta(
                g,
                &format!("Allowance maximum, {role_name}"),
                &format!(
                    "Highest voucher limit for a {role_name}. {}",
                    role(role_name)
                ),
                Some("vouchers"),
                false,
            ),
        ));
    }
    out.push(entry(
        flag(VOUCHER_REWARD_ENABLED, true, Community),
        meta(
            g,
            "Reward successful chains",
            "When a person you vouched for later vouches successfully themselves, your limit rises permanently.",
            None,
            false,
        ),
    ));
    out.push(entry(
        int(VOUCHER_REWARD_CAP, 2, 0, 10, Community),
        meta(
            g,
            "Reward cap",
            "The most extra vouchers one sponsor can earn through the reward.",
            Some("vouchers"),
            false,
        ),
    ));
    out.push(entry(
        flag(VOUCHER_SIGNOFF_SINGLE, false, Community),
        meta(
            g,
            "Sign-off on single-use links",
            "Whether the issuer must approve each person who opens a single-use link. Only admins and roots may change this default; ordinary members cannot.",
            None,
            false,
        ),
    ));
    out.push(entry(
        flag(VOUCHER_SIGNOFF_MULTI, true, Community),
        meta(
            g,
            "Sign-off on multi-use links",
            "Whether the issuer must approve each person who opens a multi-use link. Only admins and roots may change this default; ordinary members cannot.",
            None,
            false,
        ),
    ));
    out.push(entry(
        flag(VOUCHER_SPONSOR_VISIBLE, true, Community),
        meta(
            g,
            "Show who vouched",
            "Profiles say \"vouched for by <handle>\". One-way: switching it off hides every existing sponsor at once; switching it on again applies only to members admitted afterwards, never retroactively. Newcomers are told before they join.",
            None,
            true,
        ),
    ));
    out
}

fn success_rows() -> Vec<CatalogEntry> {
    let g = "Voucher success";
    let tighten = "A community may only be stricter than the platform bound.";
    vec![
        entry(
            int(SUCCESS_MIN_DAYS, 30, 14, 365, Layer::Community),
            meta(
                g,
                "Days before success",
                &format!("Days since joining before a vouchee can prove success. {tighten}"),
                Some("days"),
                false,
            ),
        ),
        entry(
            int(SUCCESS_MAX_PUNISHMENTS, 1, 0, 1, Layer::Community),
            meta(
                g,
                "Punishments allowed",
                &format!("Most punishments a successful vouchee may have received. {tighten}"),
                None,
                false,
            ),
        ),
        entry(
            int(SUCCESS_MIN_CONVERSATIONS, 3, 1, 50, Layer::Community),
            meta(
                g,
                "Answered conversations",
                "Answered conversations with distinct established members who joined before the vouchee (the sponsor does not count).",
                None,
                false,
            ),
        ),
        entry(
            int(SUCCESS_PARTNER_DAYS, 30, 14, 365, Layer::Community),
            meta(
                g,
                "Days that make a partner established",
                "How long a conversation partner must have been a member.",
                Some("days"),
                false,
            ),
        ),
        entry(
            Row {
                value_type: SettingType::Number,
                min: Some(num(0.0)),
                max: Some(num(1.0)),
                ..optional_int(SUCCESS_MIN_ACCEPTANCE, 0, 1)
            },
            meta(
                g,
                "Optional: acceptance rate",
                "Minimum share of the vouchee's own waves that were accepted. Empty means this block is off.",
                None,
                false,
            ),
        ),
        entry(
            optional_int(SUCCESS_MIN_WEEKS, 1, 52),
            meta(
                g,
                "Optional: active weeks",
                "Distinct weeks with activity. Empty means this block is off.",
                Some("weeks"),
                false,
            ),
        ),
        entry(
            optional_int(SUCCESS_MIN_STANDING, 1, 200),
            meta(
                g,
                "Optional: standing threshold",
                "Minimum messaging cap the vouchee must have reached. Empty means this block is off.",
                None,
                false,
            ),
        ),
        entry(
            optional_int(SUCCESS_MAX_GIVEN, 0, 1000),
            meta(
                g,
                "Optional: punishments given",
                "Most punishments the vouchee may have given. Empty means this block is off.",
                None,
                false,
            ),
        ),
        entry(
            flag(SUCCESS_REQUIRE_GATES, false, Layer::Community),
            meta(
                g,
                "Optional: later gates passed",
                "The vouchee must have passed further gates after joining.",
                None,
                false,
            ),
        ),
        entry(
            flag(SUCCESS_REQUIRE_VOUCHED, false, Layer::Community),
            meta(
                g,
                "Optional: has vouched successfully",
                "The vouchee must have vouched for someone successfully. Favours growth; it is pyramid-like, so it is off by default.",
                None,
                false,
            ),
        ),
    ]
}

fn community_rows() -> Vec<CatalogEntry> {
    let g = "Communities";
    let reserved: Vec<&str> = vec![
        "vault",
        "admin",
        "www",
        "api",
        "root",
        "wallet",
        "mail",
        "platform",
        "cmtymeet",
        "security",
        "official",
        "postmaster",
        "hostmaster",
        "webmaster",
        "autoconfig",
        "passport",
        "accounts",
        "settings",
        "registry",
        "support",
    ];
    vec![
        entry(
            int(CREATE_PER_WEEK, 1, 0, 100, Layer::Platform),
            meta(
                g,
                "New communities per week",
                "How many communities one person may start in a rolling seven days, counted unlinkably through the global passport.",
                Some("per 7 days"),
                false,
            ),
        ),
        entry(
            int(CREATE_PER_YEAR, 12, 0, 1000, Layer::Platform),
            meta(
                g,
                "New communities per year",
                "How many communities one person may start in a rolling 365 days.",
                Some("per 365 days"),
                false,
            ),
        ),
        entry(
            int(ADDRESS_MIN, 8, 8, 30, Layer::Platform),
            meta(
                g,
                "Shortest address",
                "Every shorter address is reserved.",
                Some("characters"),
                false,
            ),
        ),
        entry(
            int(ADDRESS_MAX, 30, 8, 63, Layer::Platform),
            meta(
                g,
                "Longest address",
                "Addresses use a-z and 0-9 only.",
                Some("characters"),
                false,
            ),
        ),
        entry(
            Row {
                key: ADDRESS_RESERVED.into(),
                value_type: SettingType::Array,
                nullable: false,
                default: json!(reserved),
                min: None,
                max: None,
                layer: Layer::Platform,
            },
            meta(
                g,
                "Reserved addresses",
                "Names that no community may take, as a list of lowercase strings. Names shorter than the minimum are reserved anyway.",
                None,
                false,
            ),
        ),
        entry(
            int(WAVES_WAITING_PERIOD, 7, 1, 30, Layer::Community),
            meta(
                "Waves",
                "Wave waiting period",
                "Days after a Close before the sender's introduction returns.",
                Some("days"),
                false,
            ),
        ),
    ]
}

/// czkp standing parameters: key suffix, label, help, default, min, max, unit.
const STANDING: &[(&str, &str, &str, u64, u64, u64, &str)] = &[
    (
        "c0",
        "Newcomer cap",
        "Open waves a newcomer may have on day one.",
        3,
        1,
        16,
        "waves",
    ),
    (
        "h_a",
        "Starter loan half-life",
        "Days until the starter loan halves.",
        14,
        1,
        64,
        "days",
    ),
    (
        "h_g",
        "Goodwill half-life",
        "Days until goodwill halves.",
        14,
        1,
        64,
        "days",
    ),
    (
        "n_half",
        "Punishment half-life",
        "Clean conversations until punishment memory halves.",
        25,
        5,
        200,
        "conversations",
    ),
    (
        "n_c",
        "Record length",
        "Length of the clean-conversation record.",
        60,
        10,
        200,
        "conversations",
    ),
    (
        "e_q",
        "Goodwill per answer",
        "Goodwill earned for answering a wave.",
        Q / 4,
        0,
        Q,
        "q16",
    ),
    (
        "r_max",
        "Maximum reach",
        "Largest number of open waves above one.",
        40,
        1,
        200,
        "waves",
    ),
    (
        "r_age",
        "Reach growth",
        "Reach ceiling growth per day of membership.",
        2,
        0,
        16,
        "per day",
    ),
    (
        "c_max_q",
        "Veteran offset",
        "Largest veteran offset, below one punishment.",
        52_428,
        0,
        58_982,
        "q16",
    ),
    (
        "c_vet",
        "Veteran record",
        "Clean-conversation record for the full veteran offset.",
        30,
        1,
        200,
        "conversations",
    ),
    (
        "age_vet",
        "Veteran age",
        "Days of membership for the full veteran offset.",
        60,
        1,
        365,
        "days",
    ),
    (
        "boost_q",
        "Newcomer boost",
        "Extra goodwill per answer in the boost window.",
        Q,
        0,
        2 * Q,
        "q16",
    ),
    (
        "boost_days",
        "Boost window",
        "Length of the boost window.",
        30,
        0,
        90,
        "days",
    ),
    (
        "slope_q",
        "Trust slope",
        "Logistic slope of the trust curve.",
        4 * Q,
        Q,
        8 * Q,
        "q16",
    ),
    (
        "mid_q",
        "Trust midpoint",
        "Excess punishments at which trust halves.",
        49_152,
        Q / 4,
        3 * Q,
        "q16",
    ),
    (
        "kappa_q",
        "Reach per goodwill",
        "Reach per unit of goodwill (signed epoch input).",
        2 * Q,
        Q / 2,
        6 * Q,
        "q16",
    ),
    (
        "loan_use_q",
        "Loan use per miss",
        "Starter loan used per unanswered wave (signed epoch input).",
        Q / 4,
        1,
        Q,
        "q16",
    ),
];

/// Key of the standing parameter with the given czkp field name.
pub fn standing_key(field: &str) -> String {
    format!("{STANDING_PREFIX}{field}")
}

fn standing_rows() -> Vec<CatalogEntry> {
    STANDING
        .iter()
        .map(|(field, label, help, default, min, max, unit)| {
            entry(
                int(
                    &standing_key(field),
                    *default as i64,
                    *min as i64,
                    *max as i64,
                    Layer::Community,
                ),
                meta("Standing", label, help, Some(unit), false),
            )
        })
        .collect()
}

/// Every platform tunable, in screen order.
pub fn platform_catalog() -> Vec<CatalogEntry> {
    let mut all = voucher_rows();
    all.extend(success_rows());
    all.extend(community_rows());
    all.extend(standing_rows());
    all
}

/// Define every tunable that the rulebook does not carry yet. Existing
/// definitions are left alone (definitions are immutable within a lineage).
pub fn define_platform_settings(book: &mut Rulebook) -> Result<()> {
    for entry in platform_catalog() {
        if !book.catalog().contains_key(&entry.key) {
            book.define(entry.key, entry.setting)?;
        }
    }
    Ok(())
}

/// Presentation text for a key; settings outside the catalogue get a plain one.
pub fn meta_for(key: &str) -> SettingMeta {
    platform_catalog()
        .into_iter()
        .find(|e| e.key == key)
        .map(|e| e.meta)
        .unwrap_or_else(|| meta("Other", key, "", None, false))
}

impl Rulebook {
    /// The generic settings screen's data: every setting that has no `/` in its
    /// key (gates, providers and action policies have their own views), with
    /// definition, layer rows and the resolved value.
    pub fn describe(&self) -> Result<Vec<SettingView>> {
        let resolved = self.resolve(&Default::default())?;
        let catalog = platform_catalog();
        let mut views = Vec::new();
        for (key, setting) in &self.catalog {
            if key.contains('/') {
                continue;
            }
            let meta = catalog
                .iter()
                .find(|e| e.key == *key)
                .map(|e| e.meta.clone())
                .unwrap_or_else(|| meta("Other", key, "", None, false));
            views.push(SettingView {
                key: key.clone(),
                meta,
                value_type: setting.value_type,
                nullable: setting.nullable,
                min: setting.bounds.min.clone(),
                max: setting.bounds.max.clone(),
                lowest_layer: setting.lowest_layer,
                kind: setting.kind,
                default: setting.default.clone(),
                platform: self.platform.get(key).cloned(),
                community: self.community.get(key).cloned(),
                effective: resolved.get(key).cloned().unwrap_or(Value::Null),
            });
        }
        let order = |key: &str| {
            catalog
                .iter()
                .position(|e| e.key == key)
                .unwrap_or(usize::MAX)
        };
        views.sort_by_key(|v| (order(&v.key), v.key.clone()));
        Ok(views)
    }
}
