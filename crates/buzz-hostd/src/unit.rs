use crate::{
    request::Request,
    store::{Catalog, Store},
};
use anyhow::{ensure, Result};
use buzz_core::paired_message::Scope;
use buzz_hostd::Instance;

use std::{fs, io::Write, path::Path};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn activate(
    store: &mut Store,
    q: &Request,
    scope: &Scope,
    oa: &str,
    cat: &Catalog,
    _revision: &str,
) -> Result<()> {
    let root = Path::new("/var/lib/buzz-hostd");
    ensure!(
        store.0.path() == root.join("host.sqlite").to_str(),
        "system service bootstrap required"
    );
    let unit_id = buzz_hostd::unit_id(scope, &q.r);
    ensure!(
        cat.current(&q.revision)?,
        crate::onboard::Permanent("catalog-changed".into())
    );
    let agent = q.agent_keys()?;
    let packet = Instance {
        deployment: None,
        unit_id: unit_id.clone(),
        scope: scope.clone(),
        runtime_id: q.runtime.clone(),
        request_id: q.r.clone(),
        revision: q.revision.clone(),
        channels: q.channels.clone(),
        agent: agent.public_key().to_hex(),
        secret: agent.secret_key().to_secret_hex(),
        oa: oa.into(),
        policy: q.policy.clone(),
    };
    let target = root.join(format!("{unit_id}.cred"));
    let bytes = serde_json::to_vec(&packet)?;
    if !target.exists() {
        let mut temp = tempfile::NamedTempFile::new_in(root)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        if let Err(e) = temp.persist_noclobber(&target) {
            if e.error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(e.error.into());
            }
        }
        fs::File::open(root)?.sync_all()?;
    }
    ensure!(
        buzz_hostd::bounded(&target, 65536)? == bytes,
        "existing instance differs; operator reconciliation required"
    );
    let prepared = action(store, "prepare", &unit_id).await?;
    if prepared == b"failed\n" {
        return Err(crate::onboard::Permanent("unit-failed".into()).into());
    }
    ensure!(prepared == b"prepared\n", "awaiting root reservation");
    let tx = store.tx()?;
    let mut local = Request::load(&tx, &scope.generation, &q.r)?
        .ok_or_else(|| anyhow::anyhow!("missing local commit"))?;
    ensure!(
        local.commit.is_some() && local.failure.is_none(),
        "create is not activatable"
    );
    local.start_attempted = true;
    local.update(&tx, &scope.generation)?;
    tx.commit()?;
    let reply = action(store, "start", &unit_id).await?;
    if reply == b"failed\n" {
        return Err(crate::onboard::Permanent("unit-failed".into()).into());
    }
    ensure!(reply == b"active\n", "local activation pending");
    Ok(())
}

pub async fn quiesce(store: &Store, q: &Request, scope: &Scope) -> Result<()> {
    let reply = action(store, "abort", &buzz_hostd::unit_id(scope, &q.r)).await?;
    ensure!(reply == b"inactive\n", "local cleanup pending");
    Ok(())
}

pub async fn settle(store: &Store, q: &Request, scope: &Scope) -> Result<()> {
    ensure!(
        action(store, "settle", &buzz_hostd::unit_id(scope, &q.r)).await? == b"settled\n",
        "root settlement pending"
    );
    Ok(())
}

async fn action(_store: &Store, verb: &str, id: &str) -> Result<Vec<u8>> {
    tokio::time::timeout(std::time::Duration::from_secs(90), async {
        let socket = Path::new("/run/buzz-hostd-activate.sock");
        #[cfg(test)]
        let socket = _store.2.as_deref().unwrap_or(socket);
        let mut sock = tokio::net::UnixStream::connect(socket).await?;
        sock.write_all(format!("{verb} {id}").as_bytes()).await?;
        sock.shutdown().await?;
        let mut reply = Vec::new();
        sock.take(16).read_to_end(&mut reply).await?;
        Ok::<_, anyhow::Error>(reply)
    })
    .await?
}
