use crlt::{Community, Db, IntoParams, Transaction, params};

use super::{Change, Revision, Selection, Storage, community_key, prepare, validate_selection};
use crate::{Error, Result, Rulebook};

/// Trusted DDL for the service's crlt migration list. Version numbering belongs
/// to the composition root so several libraries can share one migration history.
pub const SCHEMA: &str = include_str!("schema.sql");

const TABLES: [&str; 6] = [
    "setting_catalog",
    "platform_value",
    "template",
    "template_value",
    "community_template",
    "community_value",
];
const LATEST: &str = "SELECT revision, announced_at, effective_at, notice_seconds, policy_epoch FROM rulebook_revision ORDER BY revision DESC LIMIT 1";
const AT: &str = "SELECT revision, announced_at, effective_at, notice_seconds, policy_epoch FROM rulebook_revision WHERE effective_at <= ?1 ORDER BY effective_at DESC LIMIT 1";
const EXACT: &str = "SELECT revision, announced_at, effective_at, notice_seconds, policy_epoch FROM rulebook_revision WHERE revision = ?1";

/// Native libSQL backend through crlt. Credentials, connection routing and
/// migrations belong to the service; no direct libsql dependency or logging.
#[derive(Clone)]
pub struct LibsqlStore {
    db: Db,
}

impl LibsqlStore {
    /// Wrap a database after the service has applied SCHEMA through crlt.
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    fn scope(&self, community: &str) -> Result<Community> {
        community_key(community)?;
        self.db.community(community).map_err(|_| Error::Storage)
    }

    /// Assert index plans for every read shape. crlt also checks each statement
    /// before executing it, including inserts and transaction reads.
    pub async fn check_query_plans(&self, community: &str) -> Result<()> {
        let scope = self.scope(community)?;
        check_plan(&scope, LATEST, ()).await?;
        for query in [AT, EXACT] {
            check_plan(&scope, query, [1_i64]).await?;
        }
        for table in TABLES {
            check_plan(&scope, &read_sql(table), [1_i64]).await?;
        }
        Ok(())
    }
}

async fn check_plan(scope: &Community, query: &str, params: impl IntoParams) -> Result<()> {
    scope
        .explain(query, params)
        .await
        .map_err(|_| Error::Storage)?
        .assert_indexed()
        .map_err(|_| Error::Storage)
}

impl Storage for LibsqlStore {
    async fn load(&self, community: &str, selection: Selection) -> Result<Option<Revision>> {
        validate_selection(selection)?;
        let mut tx = self
            .scope(community)?
            .tx()
            .await
            .map_err(|_| Error::Storage)?;
        let result = read(&mut tx, selection).await?;
        tx.commit().await.map_err(|_| Error::Storage)?;
        Ok(result)
    }

    async fn append(
        &self,
        community: &str,
        expected: Option<u64>,
        change: Change,
    ) -> Result<Revision> {
        let mut tx = self
            .scope(community)?
            .tx()
            .await
            .map_err(|_| Error::Storage)?;
        let previous = read(&mut tx, Selection::Latest).await?;
        let next = prepare(previous.as_ref(), expected, change)?;
        tx.execute("INSERT INTO rulebook_revision (revision, announced_at, effective_at, notice_seconds, policy_epoch) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![next.revision as i64, next.change.announced_at, next.change.effective_at, next.change.notice_seconds as i64, next.change.policy_epoch as i64])
            .await.map_err(|_| Error::Storage)?;
        write(&mut tx, &next).await?;
        tx.commit().await.map_err(|_| Error::Storage)?;
        Ok(next)
    }
}

fn read_sql(table: &str) -> String {
    format!("SELECT entry_key, payload FROM {table} WHERE revision = ?1")
}

async fn read(tx: &mut Transaction, selection: Selection) -> Result<Option<Revision>> {
    let rows = match selection {
        Selection::Latest => tx.query(LATEST, ()).await,
        Selection::At(time) => tx.query(AT, [time]).await,
        Selection::Revision(revision) => tx.query(EXACT, [revision as i64]).await,
    }
    .map_err(|_| Error::Storage)?;
    let Some(row) = rows.first() else {
        return Ok(None);
    };
    let field = |index| row.get_i64(index).map_err(|_| Error::Corrupt);
    let revision = u64::try_from(field(0)?).map_err(|_| Error::Corrupt)?;
    let mut change = Change {
        rulebook: Rulebook::default(),
        announced_at: field(1)?,
        effective_at: field(2)?,
        notice_seconds: u64::try_from(field(3)?).map_err(|_| Error::Corrupt)?,
        policy_epoch: u64::try_from(field(4)?).map_err(|_| Error::Corrupt)?,
    };
    for table in TABLES {
        let rows = tx
            .query(&read_sql(table), [revision as i64])
            .await
            .map_err(|_| Error::Storage)?;
        for row in rows {
            let key = row.get_str(0).map_err(|_| Error::Corrupt)?;
            let payload = row.get_str(1).map_err(|_| Error::Corrupt)?;
            let book = &mut change.rulebook;
            match table {
                "setting_catalog" => {
                    book.catalog.insert(key.into(), decode(payload)?);
                }
                "platform_value" => {
                    book.platform.insert(key.into(), decode(payload)?);
                }
                "template" => {
                    let (name, version) = template_key(key)?;
                    book.templates
                        .entry(name.into())
                        .or_default()
                        .insert(version, decode(payload)?);
                }
                "template_value" => {
                    let (pin, setting) = key.split_once('/').ok_or(Error::Corrupt)?;
                    let (name, version) = template_key(pin)?;
                    book.templates
                        .get_mut(name)
                        .and_then(|v| v.get_mut(&version))
                        .ok_or(Error::Corrupt)?
                        .values
                        .insert(setting.into(), decode(payload)?);
                }
                "community_template" if key == "pin" && book.pin.is_none() => {
                    book.pin = Some(decode(payload)?)
                }
                "community_value" => {
                    book.community.insert(key.into(), decode(payload)?);
                }
                _ => return Err(Error::Corrupt),
            }
        }
    }
    if revision == 0 {
        return Err(Error::Corrupt);
    }
    prepare(None, None, change.clone()).map_err(|_| Error::Corrupt)?;
    Ok(Some(Revision { revision, change }))
}

fn template_key(key: &str) -> Result<(&str, u64)> {
    let (name, version) = key.split_once('@').ok_or(Error::Corrupt)?;
    Ok((name, version.parse().map_err(|_| Error::Corrupt)?))
}

fn decode<T: serde::de::DeserializeOwned>(payload: &str) -> Result<T> {
    serde_json::from_str(payload).map_err(|_| Error::Corrupt)
}

async fn insert(
    tx: &mut Transaction,
    revision: u64,
    table: &str,
    key: &str,
    payload: &(impl serde::Serialize + Sync),
) -> Result<()> {
    let payload = serde_json::to_string(payload).map_err(|_| Error::Invalid("serialization"))?;
    tx.execute(
        &format!("INSERT INTO {table} (revision, entry_key, payload) VALUES (?1, ?2, ?3)"),
        params![revision as i64, key, payload],
    )
    .await
    .map_err(|_| Error::Storage)?;
    Ok(())
}

async fn write(tx: &mut Transaction, revision: &Revision) -> Result<()> {
    let number = revision.revision;
    let book = &revision.change.rulebook;
    for (key, value) in &book.catalog {
        insert(tx, number, "setting_catalog", key, value).await?;
    }
    for (key, value) in &book.platform {
        insert(tx, number, "platform_value", key, value).await?;
    }
    for (name, versions) in &book.templates {
        for (version, template) in versions {
            let pin = format!("{name}@{version}");
            let base = crate::settings::Template {
                base: template.base.clone(),
                values: Default::default(),
            };
            insert(tx, number, "template", &pin, &base).await?;
            for (key, value) in &template.values {
                insert(tx, number, "template_value", &format!("{pin}/{key}"), value).await?;
            }
        }
    }
    if let Some(pin) = &book.pin {
        insert(tx, number, "community_template", "pin", pin).await?;
    }
    for (key, value) in &book.community {
        insert(tx, number, "community_value", key, value).await?;
    }
    Ok(())
}
