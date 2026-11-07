//! Disposable registration driver. Generates its own isolated signer; no normal
//! desktop key import/export. DIR HTTPS_ORIGIN init|hosts|pair|unpair GENERATION.
use anyhow::{bail, ensure, Context, Result};
use buzz_paired_owner::{registry::Registry, Device};
use buzz_ws_client::NostrWsConnection;
use nostr::{Keys, SecretKey};
use std::{
    env, fs,
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
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
        .context("init, hosts, pair, or unpair required")?;
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
    let keys = signer()?;
    let pem = env::var_os("BUZZ_REGISTRATION_CA")
        .map(fs::read)
        .transpose()?;
    let mut client = reqwest::Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10));
    if let Some(pem) = &pem {
        client = client.add_root_certificate(reqwest::Certificate::from_pem(pem)?);
    }
    let metadata: serde_json::Value = client
        .build()?
        .get(&origin)
        .header("Accept", "application/nostr+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let relay = Registry::capability(&metadata)?;
    let url = origin.replacen("https://", "wss://", 1);
    let mut ws = if let Some(pem) = &pem {
        use rustls::pki_types::pem::PemObject;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls::pki_types::CertificateDer::pem_slice_iter(pem) {
            roots.add(cert?)?;
        }
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        NostrWsConnection::connect_with_tls_connector(
            &url,
            buzz_ws_client::Connector::Rustls(Arc::new(config)),
        )
        .await?
    } else {
        NostrWsConnection::connect(&url).await?
    };
    ws.authenticate(&keys, None).await?;
    let mut net = Registry::from_authenticated(&origin, keys.public_key(), relay, ws)?;
    let mut device = Device::at(&dir.join("owner.sqlite"))?;
    match op.as_str() {
        "pair" => {
            let (scope, event) = device.pair_registry(&origin, &keys, &stdin_text()?)?;
            net.publish(&keys, event).await?;
            println!(
                "{}",
                serde_json::to_string(&device.view(&origin, &keys, &scope)?)?
            );
        }
        "hosts" => {
            net.recover_guarded(&mut device, &signer).await?;
            println!(
                "{}",
                serde_json::to_string(&device.registry_hosts(&origin, &keys)?)?
            );
        }
        "unpair" => {
            net.recover_guarded(&mut device, &signer).await?;
            let generation = args.next().context("generation required")?;
            let card = device
                .registry_hosts(&origin, &keys)?
                .into_iter()
                .find(|card| card.scope.generation == generation)
                .context("registration missing")?;
            let event = device.unpair_registry(&origin, &keys, &card.scope)?;
            net.publish(&keys, event).await?;
            println!(
                "{}",
                serde_json::to_string(&device.view(&origin, &keys, &card.scope)?)?
            );
        }
        _ => bail!("unknown registration operation"),
    }
    net.disconnect().await?;
    Ok(())
}
