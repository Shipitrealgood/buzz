//! Reconcile the two deployed paired migrations that used upstream version numbers.
//! All checks and the optional transfer run on the schema lock's own backend.

use sqlx::migrate::{Migrate, Migration, Migrator};
use sqlx::{Connection, PgConnection};

use super::{EXTENSION_MIGRATOR, MIGRATOR};
use crate::{DbError, Result};

const LAST_SHARED_VERSION: i64 = 80;
const LAST_COLLISION: i64 = 82;

#[derive(sqlx::FromRow)]
struct Receipt {
    version: i64,
    description: String,
    checksum: Vec<u8>,
    success: bool,
}

impl Receipt {
    fn matches(&self, migration: &Migration) -> bool {
        self.success
            && self.version == migration.version
            && self.description == migration.description
            && self.checksum.as_slice() == migration.checksum.as_ref()
    }
}

fn validate_prefix(rows: &[Receipt], source: &Migrator, ledger: &str) -> Result<()> {
    let mut expected = source.iter();
    for row in rows {
        if !expected
            .next()
            .is_some_and(|migration| row.matches(migration))
        {
            return Err(DbError::InvalidData(format!(
                "{ledger} migration history is not an exact successful prefix at version {}",
                row.version
            )));
        }
    }
    Ok(())
}

/// Identify the single historical split. The fork shipped through 81 and then
/// through 82. After transfer the ordinary upstream prefix can legitimately end
/// anywhere from 80 onward (including a restart after a failed upstream step).
fn legacy_receipts<'a>(main: &'a [Receipt], extensions: &[Receipt]) -> Result<&'a [Receipt]> {
    let first_extension = EXTENSION_MIGRATOR
        .iter()
        .next()
        .ok_or_else(|| DbError::InvalidData("missing embedded paired hosts migration".into()))?;
    let (upstream, legacy) = if main.iter().any(|row| row.matches(first_extension)) {
        main.split_at(main.partition_point(|row| row.version <= LAST_SHARED_VERSION))
    } else {
        (main, &[][..])
    };
    validate_prefix(upstream, &MIGRATOR, "upstream")?;
    validate_prefix(extensions, &EXTENSION_MIGRATOR, "extension")?;
    if !legacy.is_empty() {
        if !extensions.is_empty() || legacy.last().is_some_and(|r| r.version > LAST_COLLISION) {
            return Err(DbError::InvalidData(
                "mixed legacy and extension migration histories".into(),
            ));
        }
        validate_prefix(legacy, &EXTENSION_MIGRATOR, "legacy fork")?;
    }
    if (!legacy.is_empty() || !extensions.is_empty())
        && upstream
            .last()
            .is_none_or(|r| r.version < LAST_SHARED_VERSION)
    {
        return Err(DbError::InvalidData(
            "paired migration history lacks the shared upstream prefix".into(),
        ));
    }
    Ok(legacy)
}

pub(super) async fn reconcile(conn: &mut PgConnection) -> Result<()> {
    let mut tx = conn.begin().await?;
    // The frozen paired-host DDL creates unqualified tables. The upstream
    // migrations also mix public-qualified and unqualified names.
    let targets_public: bool =
        sqlx::query_scalar("SELECT current_schema() IS NOT DISTINCT FROM 'public'")
            .fetch_one(&mut *tx)
            .await?;
    if !targets_public {
        return Err(DbError::InvalidData(
            "Buzz migrations require public as the current schema".into(),
        ));
    }
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    tx.ensure_migrations_table("public._sqlx_migrations")
        .await?;
    tx.ensure_migrations_table(&EXTENSION_MIGRATOR.table_name)
        .await?;
    sqlx::query("LOCK TABLE public._sqlx_migrations, public._buzz_extension_migrations IN SHARE ROW EXCLUSIVE MODE")
        .execute(&mut *tx).await?;
    let main = sqlx::query_as("SELECT version, description, checksum, success FROM public._sqlx_migrations ORDER BY version")
        .fetch_all(&mut *tx).await?;
    let extensions = sqlx::query_as("SELECT version, description, checksum, success FROM public._buzz_extension_migrations ORDER BY version")
        .fetch_all(&mut *tx).await?;
    let legacy = legacy_receipts(&main, &extensions)?;
    let paired_version = legacy.last().or(extensions.last()).map_or(0, |r| r.version);

    // Neither an existing untracked table nor a missing tracked table is a
    // reason to adopt/replay its DDL. Check all six names before any transfer.
    let mismatch: Option<String> = sqlx::query_scalar(
        "SELECT expected.name FROM (VALUES
            ('paired_host_events', 81), ('paired_host_bindings', 81), ('paired_host_requests', 81),
            ('paired_agent_placements', 82), ('paired_agent_operations', 82), ('paired_host_deployment_status', 82)
         ) AS expected(name, version)
         LEFT JOIN pg_catalog.pg_class c ON c.relnamespace = 'public'::regnamespace AND c.relname = expected.name
         WHERE CASE WHEN expected.version <= $1::bigint THEN c.relkind IS DISTINCT FROM 'r'
                    ELSE c.oid IS NOT NULL END LIMIT 1",
    )
    .bind(paired_version)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(name) = mismatch {
        return Err(DbError::InvalidData(format!(
            "paired table public.{name} disagrees with migration history"
        )));
    }
    if !legacy.is_empty() {
        let versions: Vec<_> = legacy.iter().map(|r| r.version).collect();
        let moved = sqlx::query(
            "WITH moved AS (
                DELETE FROM public._sqlx_migrations WHERE version = ANY($1)
                RETURNING version, description, installed_on, success, checksum, execution_time
             )
             INSERT INTO public._buzz_extension_migrations
                 (version, description, installed_on, success, checksum, execution_time)
             SELECT version, description, installed_on, success, checksum, execution_time FROM moved",
        )
        .bind(&versions)
        .execute(&mut *tx)
        .await?;
        if moved.rows_affected() != legacy.len() as u64 {
            return Err(DbError::InvalidData(
                "incomplete paired migration history transfer".into(),
            ));
        }
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipts(source: &Migrator) -> Vec<Receipt> {
        source
            .iter()
            .map(|m| Receipt {
                version: m.version,
                description: m.description.to_string(),
                checksum: m.checksum.to_vec(),
                success: true,
            })
            .collect()
    }

    #[test]
    fn transfer_only_the_exact_deployed_prefix() {
        let canonical = receipts(&MIGRATOR);
        let extensions = receipts(&EXTENSION_MIGRATOR);
        let base_len = canonical.partition_point(|r| r.version <= LAST_SHARED_VERSION);

        // Every genuine canonical prefix (including fresh and current) runs
        // through normal SQLx. A transferred prefix is also restartable.
        for end in 0..=canonical.len() {
            assert!(legacy_receipts(&canonical[..end], &[]).unwrap().is_empty());
            assert_eq!(
                legacy_receipts(&canonical[..end], &extensions).is_ok(),
                end >= base_len
            );
        }
        for deployed in [1, 2] {
            let mut legacy = receipts(&MIGRATOR);
            legacy.truncate(base_len);
            legacy.extend(receipts(&EXTENSION_MIGRATOR).into_iter().take(deployed));
            assert_eq!(legacy_receipts(&legacy, &[]).unwrap().len(), deployed);
            assert!(legacy_receipts(&legacy, &extensions[..1]).is_err());
            legacy[base_len].success = false;
            assert!(legacy_receipts(&legacy, &[]).is_err());
        }
        let mut mixed = receipts(&MIGRATOR);
        mixed[base_len] = receipts(&EXTENSION_MIGRATOR).remove(0);
        assert!(legacy_receipts(&mixed, &[]).is_err());

        let mut corrupted = receipts(&MIGRATOR);
        corrupted[0].checksum.reverse();
        assert!(legacy_receipts(&corrupted, &[]).is_err());
        let mut corrupted = receipts(&EXTENSION_MIGRATOR);
        corrupted[1].description.push('!');
        assert!(legacy_receipts(&canonical, &corrupted).is_err());
        assert!(legacy_receipts(&canonical, &extensions[1..]).is_err());
    }
}
