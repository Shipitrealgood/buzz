//! Disposable driver for the exact native Windows owner protocol. Never imports a normal
//! client signer; the only supported identity is generated in a dedicated private directory.
//! DIR HTTPS_ORIGIN init | hosts | pair (host console JSON on stdin) |
//! create GENERATION (Draft JSON on stdin) | resume GENERATION REQUEST | unpair GENERATION.
use anyhow::{bail, ensure, Context, Result};
use buzz_paired_owner::{Device, Draft, Endpoint, Scope};
use nostr::{Keys, SecretKey};
use std::{
    env, fs,
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};
use zeroize::Zeroizing;

fn stdin_text() -> Result<Zeroizing<String>> {
    let mut s = Zeroizing::new(String::new());
    std::io::stdin().take(4097).read_to_string(&mut s)?;
    ensure!(s.len() <= 4096, "input too large");
    Ok(s)
}
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let dir = PathBuf::from(args.next().context("private staging directory required")?);
    let origin = args
        .next()
        .context("canonical staging HTTPS origin required")?;
    let op = args
        .next()
        .context("init, hosts, pair, create, resume, or unpair required")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let m = fs::symlink_metadata(&dir)?;
        ensure!(
            m.is_dir() && m.permissions().mode() & 0o077 == 0,
            "use a private 0700 directory"
        );
    }
    #[cfg(windows)]
    {
        let base = PathBuf::from(
            env::var_os("LOCALAPPDATA").context("local application data unavailable")?,
        );
        buzz_paired_owner::protect_windows_journal(&base, &dir)?;
    }
    let keyfile = dir.join("disposable-owner.secret");
    if op == "init" {
        let keys = Keys::generate();
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(keyfile)?;
        f.write_all(Zeroizing::new(keys.secret_key().to_secret_hex()).as_bytes())?;
        f.sync_all()?;
        println!("{}", keys.public_key().to_hex());
        return Ok(());
    }
    let signer = || -> Result<Keys> {
        Ok(Keys::new(
            SecretKey::from_hex(&*Zeroizing::new(fs::read_to_string(&keyfile)?))
                .map_err(|_| anyhow::anyhow!("disposable signer unavailable"))?,
        ))
    };
    let owner = signer()?.public_key();
    let net = if let Some(pem) = env::var_os("BUZZ_PAIRED_STAGING_CA") {
        Endpoint::staging(
            &origin,
            owner,
            reqwest::Certificate::from_pem(&fs::read(pem)?)?,
        )?
    } else {
        Endpoint::new(&origin, owner)?
    };
    ensure!(
        net.supported().await?,
        "community does not advertise paired hosts v1"
    );
    let mut device = Device::at(&dir.join("paired-owner.db"))?;
    if op == "pair" {
        let (s, p) = device.pair(&origin, &signer()?, &stdin_text()?)?;
        net.claim(&signer()?, &s, &p).await?;
        for _ in 0..60 {
            if net.binding(&signer()?, &s).await.is_ok() {
                let view = net.refresh(&mut device, &signer, &s).await?;
                println!("{}", serde_json::to_string(&view)?);
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        bail!("pair claim pending")
    }
    let mut scopes = vec![];
    for b in net.hosts(&signer).await? {
        let s = device.restore(&origin, &signer()?, &b)?;
        if op == "hosts" {
            println!(
                "{}",
                serde_json::to_string(&device.view(&origin, &signer()?, &s)?)?
            );
        }
        scopes.push(s);
    }
    if op == "hosts" {
        return Ok(());
    }
    let g = args.next().context("generation required")?;
    let s: Scope = scopes
        .into_iter()
        .find(|s| s.generation == g)
        .context("host unavailable")?;
    let r = match op.as_str() {
        "create" => {
            let draft: Draft = serde_json::from_str(&stdin_text()?)
                .map_err(|_| anyhow::anyhow!("invalid draft"))?;
            net.refresh(&mut device, &signer, &s).await?;
            let catalog = device.catalog_id(&origin, &signer()?, &s)?.to_owned();
            let r = net.start(&mut device, &signer, &s, &catalog, draft).await?;
            println!("{}", serde_json::json!({"request_id":r,"state":"pending"}));
            r
        }
        "resume" => args.next().context("request ID required")?,
        "unpair" => {
            let p = device.unpair(&origin, &signer()?, &s)?;
            net.post(&signer()?, &s, &p).await?;
            for _ in 0..60 {
                let b = net.binding(&signer()?, &s).await?;
                device.restore(&origin, &signer()?, &b)?;
                let view = device.view(&origin, &signer()?, &s)?;
                if view.state == "unpaired" {
                    println!("{}", serde_json::to_string(&view)?);
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            bail!("unpair pending at host")
        }
        _ => bail!("unknown staging operation"),
    };
    let mut last = String::new();
    for _ in 0..120 {
        match net.reconcile(&mut device, &signer, &s, &r).await {
            Ok(view) => {
                if last != view.state {
                    println!("{}", serde_json::to_string(&view)?);
                    last = view.state;
                }
                if last == "ready" {
                    return Ok(());
                }
                if matches!(
                    last.as_str(),
                    "failed" | "revoked-before-commit" | "expired"
                ) {
                    bail!("host returned terminal failure");
                }
            }
            Err(e) => {
                eprintln!("pending: {e}");
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    bail!("request remains pending; resume using the same device and request ID")
}
