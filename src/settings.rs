use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ActionPolicy, Error, Result, identifier};

/// A sparse set of whole JSON values. Absence inherits; JSON null terminates.
pub type Values = BTreeMap<String, Value>;

/// Layers in descending authority order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    /// Platform operator.
    Platform,
    /// Published template version.
    Template,
    /// Community administrator.
    Community,
    /// Transient member latitude.
    Member,
}

/// Whether template values are pinned or platform fallbacks remain live.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingKind {
    /// Ignores templates; follows platform changes unless overridden.
    Technical,
    /// Uses the community's explicitly pinned template version.
    Template,
}

/// Supported JSON types; null is accepted separately when nullable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingType {
    /// JSON boolean.
    Boolean,
    /// Signed or unsigned JSON integer, with exact integer bounds.
    Integer,
    /// Finite JSON number, with floating point bounds.
    Number,
    /// JSON string.
    String,
    /// Whole JSON array (never merged).
    Array,
    /// Whole JSON object (never merged).
    Object,
    /// A validated action policy.
    Policy,
}

/// Numeric inclusive bounds. Integer settings require integer bounds.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    /// Inclusive minimum.
    pub min: Option<serde_json::Number>,
    /// Inclusive maximum.
    pub max: Option<serde_json::Number>,
}

/// Catalogue metadata. Definitions are immutable within a rulebook lineage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setting {
    /// Required non-null value type.
    pub value_type: SettingType,
    /// Whether explicit null is allowed.
    pub nullable: bool,
    /// Fallback when every applicable layer is absent.
    pub default: Value,
    /// Numeric limits, checked at every layer including defaults.
    pub bounds: Bounds,
    /// Deepest permitted layer.
    pub lowest_layer: Layer,
    /// Live technical or pinned template setting.
    pub kind: SettingKind,
}

fn integer(value: &serde_json::Number) -> Option<i128> {
    value
        .as_i64()
        .map(i128::from)
        .or_else(|| value.as_u64().map(i128::from))
}

impl Setting {
    pub(crate) fn validate(&self) -> Result<()> {
        let bounds = [&self.bounds.min, &self.bounds.max];
        if bounds.iter().any(|v| v.is_some()) {
            if !matches!(self.value_type, SettingType::Integer | SettingType::Number) {
                return Err(Error::Invalid("bounds on nonnumeric setting"));
            }
            if self.value_type == SettingType::Integer
                && bounds.into_iter().flatten().any(|v| integer(v).is_none())
            {
                return Err(Error::Invalid("noninteger bounds"));
            }
            if let (Some(min), Some(max)) = (&self.bounds.min, &self.bounds.max) {
                let reversed = if self.value_type == SettingType::Integer {
                    integer(min) > integer(max)
                } else {
                    min.as_f64() > max.as_f64()
                };
                if reversed {
                    return Err(Error::Invalid("reversed bounds"));
                }
            }
        }
        self.check(&self.default)
    }

    /// Validate a whole value, including explicit null and numeric bounds.
    pub fn check(&self, value: &Value) -> Result<()> {
        if value.is_null() {
            return if self.nullable {
                Ok(())
            } else {
                Err(Error::Invalid("null forbidden"))
            };
        }
        let valid = match self.value_type {
            SettingType::Boolean => value.is_boolean(),
            SettingType::Integer => value.as_number().and_then(integer).is_some(),
            SettingType::Number => value.is_number(),
            SettingType::String => value.is_string(),
            SettingType::Array => value.is_array(),
            SettingType::Object => value.is_object(),
            SettingType::Policy => serde_json::from_value::<ActionPolicy>(value.clone())
                .is_ok_and(|policy| policy.validate().is_ok()),
        };
        if !valid {
            return Err(Error::Invalid("value type"));
        }
        if let Some(number) = value.as_number() {
            let outside = if self.value_type == SettingType::Integer {
                let n = integer(number).ok_or(Error::Invalid("integer"))?;
                self.bounds
                    .min
                    .as_ref()
                    .and_then(integer)
                    .is_some_and(|min| n < min)
                    || self
                        .bounds
                        .max
                        .as_ref()
                        .and_then(integer)
                        .is_some_and(|max| n > max)
            } else {
                let n = number.as_f64().ok_or(Error::Invalid("number"))?;
                self.bounds
                    .min
                    .as_ref()
                    .and_then(|v| v.as_f64())
                    .is_some_and(|min| n < min)
                    || self
                        .bounds
                        .max
                        .as_ref()
                        .and_then(|v| v.as_f64())
                        .is_some_and(|max| n > max)
            };
            if outside {
                return Err(Error::Invalid("value outside bounds"));
            }
        }
        Ok(())
    }
}

/// A platform value, including its independent force switch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformValue {
    /// Whole JSON value; null is not absence.
    pub value: Value,
    /// Override every lower layer while retaining those deviations.
    pub force: bool,
}

/// An immutable named template version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplatePin {
    /// Stable template name.
    pub name: String,
    /// Explicit positive version; never an implicit latest pointer.
    pub version: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Template {
    pub base: Values,
    pub values: Values,
}

/// A draft of community configuration, containing no member data.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rulebook {
    pub(crate) catalog: BTreeMap<String, Setting>,
    pub(crate) platform: BTreeMap<String, PlatformValue>,
    pub(crate) templates: BTreeMap<String, BTreeMap<u64, Template>>,
    pub(crate) pin: Option<TemplatePin>,
    pub(crate) community: Values,
}

impl Rulebook {
    /// Add a definition, rejecting replacement and invalid defaults.
    pub fn define(&mut self, key: impl Into<String>, setting: Setting) -> Result<()> {
        let key = key.into();
        validate_key(&key, &setting)?;
        setting.validate()?;
        if self.catalog.contains_key(&key) {
            return Err(Error::Invalid("setting already defined"));
        }
        self.catalog.insert(key, setting);
        Ok(())
    }

    /// Read the setting catalogue.
    pub fn catalog(&self) -> &BTreeMap<String, Setting> {
        &self.catalog
    }

    fn check_layer(&self, key: &str, value: &Value, layer: Layer) -> Result<()> {
        let setting = self
            .catalog
            .get(key)
            .ok_or(Error::Invalid("unknown setting"))?;
        if layer > setting.lowest_layer {
            return Err(Error::Invalid("override layer forbidden"));
        }
        if layer == Layer::Template && setting.kind == SettingKind::Technical {
            return Err(Error::Invalid("technical setting in template"));
        }
        setting.check(value)
    }

    /// Set a platform row. `None` deletes it; `Some` containing null stores null.
    pub fn set_platform(&mut self, key: &str, value: Option<PlatformValue>) -> Result<()> {
        if let Some(value) = value {
            self.check_layer(key, &value.value, Layer::Platform)?;
            self.platform.insert(key.into(), value);
        } else {
            self.catalog
                .get(key)
                .ok_or(Error::Invalid("unknown setting"))?;
            self.platform.remove(key);
        }
        Ok(())
    }

    /// Set a community deviation or remove the row with `None`.
    pub fn set_community(&mut self, key: &str, value: Option<Value>) -> Result<()> {
        let definition = self
            .catalog
            .get(key)
            .ok_or(Error::Invalid("unknown setting"))?;
        if definition.lowest_layer < Layer::Community {
            return Err(Error::Invalid("override layer forbidden"));
        }
        if let Some(value) = value {
            self.check_layer(key, &value, Layer::Community)?;
            self.community.insert(key.into(), value);
        } else {
            self.community.remove(key);
        }
        Ok(())
    }

    /// Publish once. Capture template-kind platform/default fallbacks separately
    /// from sparse deviations, so later platform edits cannot silently rebase it.
    pub fn publish_template(&mut self, pin: TemplatePin, values: Values) -> Result<()> {
        identifier(&pin.name)?;
        if pin.version == 0
            || self
                .templates
                .get(&pin.name)
                .is_some_and(|v| v.contains_key(&pin.version))
        {
            return Err(Error::Invalid("template version"));
        }
        for (key, value) in &values {
            self.check_layer(key, value, Layer::Template)?;
        }
        let base = self
            .catalog
            .iter()
            .filter(|(_, s)| s.kind == SettingKind::Template && s.lowest_layer >= Layer::Template)
            .map(|(key, s)| {
                (
                    key.clone(),
                    self.platform
                        .get(key)
                        .map_or(&s.default, |p| &p.value)
                        .clone(),
                )
            })
            .collect();
        self.templates
            .entry(pin.name)
            .or_default()
            .insert(pin.version, Template { base, values });
        Ok(())
    }

    /// Select an existing immutable template; `None` unpins it explicitly.
    pub fn pin_template(&mut self, pin: Option<TemplatePin>) -> Result<()> {
        if let Some(pin) = &pin {
            self.template(pin)?;
        }
        self.pin = pin;
        Ok(())
    }

    fn template(&self, pin: &TemplatePin) -> Result<&Template> {
        self.templates
            .get(&pin.name)
            .and_then(|v| v.get(&pin.version))
            .ok_or(Error::NotFound)
    }

    /// Resolve whole values. Even shadowed member values must be valid latitude.
    pub fn resolve(&self, member: &Values) -> Result<Values> {
        self.validate()?;
        for (key, value) in member {
            self.check_layer(key, value, Layer::Member)?;
        }
        let template = self
            .pin
            .as_ref()
            .map(|pin| self.template(pin))
            .transpose()?;
        Ok(self
            .catalog
            .iter()
            .map(|(key, setting)| {
                let platform = self.platform.get(key);
                let value = if let Some(forced) = platform.filter(|p| p.force) {
                    &forced.value
                } else {
                    member
                        .get(key)
                        .or_else(|| self.community.get(key))
                        .or_else(|| {
                            template.and_then(|t| t.values.get(key).or_else(|| t.base.get(key)))
                        })
                        .or_else(|| platform.map(|p| &p.value))
                        .unwrap_or(&setting.default)
                };
                (key.clone(), value.clone())
            })
            .collect())
    }

    /// Validate deserialized drafts as strictly as method-created drafts.
    pub fn validate(&self) -> Result<()> {
        for (key, setting) in &self.catalog {
            validate_key(key, setting)?;
            setting.validate()?;
        }
        for (key, value) in &self.platform {
            self.check_layer(key, &value.value, Layer::Platform)?;
        }
        for (key, value) in &self.community {
            self.check_layer(key, value, Layer::Community)?;
        }
        for (name, versions) in &self.templates {
            identifier(name)?;
            for (version, template) in versions {
                if *version == 0 {
                    return Err(Error::Invalid("template version"));
                }
                for (key, value) in template.base.iter().chain(&template.values) {
                    self.check_layer(key, value, Layer::Template)?;
                }
            }
        }
        if let Some(pin) = &self.pin {
            self.template(pin)?;
        }
        Ok(())
    }

    pub(crate) fn follows(&self, previous: &Self) -> Result<()> {
        for (key, setting) in &previous.catalog {
            if self.catalog.get(key) != Some(setting) {
                return Err(Error::Invalid("catalogue definition is immutable"));
            }
        }
        for (name, versions) in &previous.templates {
            for (version, template) in versions {
                if self.templates.get(name).and_then(|v| v.get(version)) != Some(template) {
                    return Err(Error::Invalid("published template is immutable"));
                }
            }
        }
        Ok(())
    }
}

fn validate_key(key: &str, setting: &Setting) -> Result<()> {
    let parts: Vec<_> = key.split('/').collect();
    for part in &parts {
        identifier(part)?;
    }
    let control = match parts.as_slice() {
        ["gate", level, _] | ["provider", level, _, _]
            if *level == "global" || *level == "community" =>
        {
            Some(SettingType::Boolean)
        }
        ["action", _] => Some(SettingType::Policy),
        ["gate" | "provider" | "action", ..] => return Err(Error::Invalid("control setting key")),
        _ => None,
    };
    if let Some(expected) = control
        && (setting.value_type != expected || setting.lowest_layer == Layer::Member)
    {
        return Err(Error::Invalid("control setting type or authority"));
    }
    Ok(())
}
