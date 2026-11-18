//! Opt-in readiness for the isolated paired-agent system unit. The launcher
//! starts one eager ACP slot. This relay task is constructed only after that
//! adapter initialized. READY comes from the main process of this invocation.
use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    os::linux::net::SocketAddrExt,
    os::unix::{
        ffi::OsStrExt,
        net::{SocketAddr, UnixDatagram},
    },
};
use uuid::Uuid;

pub(super) struct Startup {
    socket: OsString,
    required: HashSet<String>,
    confirmed: HashSet<String>,
}
impl Startup {
    pub(super) fn configured() -> Option<Self> {
        Self::new(
            std::env::var_os("NOTIFY_SOCKET"),
            std::env::var("BUZZ_PAIRED_STARTUP_CHANNELS")
                .ok()
                .as_deref(),
        )
    }
    fn new(socket: Option<OsString>, channels: Option<&str>) -> Option<Self> {
        let socket = socket.filter(|s| !s.is_empty())?;
        let ids: Vec<Uuid> = serde_json::from_str(channels?).ok()?;
        if ids.len() > 8 {
            return None;
        }
        let mut required: HashSet<_> = ids.into_iter().map(super::channel_sub_id).collect();
        required.insert(super::MEMBERSHIP_NOTIF_SUB_ID.into());
        Some(Self {
            socket,
            required,
            confirmed: HashSet::new(),
        })
    }
    pub(super) fn invalidate(&mut self, id: &str) {
        self.confirmed.remove(id);
    }
    pub(super) fn reset(&mut self) {
        self.confirmed.clear();
    }
    pub(super) fn eose(&mut self, id: &str) -> bool {
        if !self.required.contains(id) {
            return false;
        }
        self.confirmed.insert(id.to_owned());
        self.confirmed == self.required && notify(&self.socket).is_ok()
    }
}
fn notify(path: &OsStr) -> std::io::Result<()> {
    let socket = UnixDatagram::unbound()?;
    let address = match path.as_bytes().strip_prefix(b"@") {
        Some(name) => SocketAddr::from_abstract_name(name)?,
        None => SocketAddr::from_pathname(path)?,
    };
    socket.send_to_addr(b"READY=1", &address)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_and_current_connection_only() {
        assert!(Startup::new(None, Some("[]")).is_none());
        assert!(Startup::new(Some("fake".into()), None).is_none());
        let id = Uuid::new_v4();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notify");
        let sock = UnixDatagram::bind(&path).unwrap();
        let mut connection = super::super::BgState::new();
        connection.paired_startup = Startup::new(
            Some(path.into()),
            Some(&serde_json::to_string(&[id]).unwrap()),
        );
        let ch = super::super::channel_sub_id(id);
        let membership = super::super::MEMBERSHIP_NOTIF_SUB_ID;
        connection.note_eose(&ch); // unsolicited EOSE cannot count
        connection.note_req_sent(membership);
        connection.note_eose(membership);
        assert!(connection.paired_startup.is_some());
        connection.note_connection_closed();
        connection.note_req_sent(&ch);
        connection.note_eose(&ch);
        assert!(connection.paired_startup.is_some()); // old socket membership was invalidated
        connection.note_req_sent(membership);
        connection.note_eose(membership);
        assert!(connection.paired_startup.is_none());
        sock.set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .unwrap();
        let mut bytes = [0; 16];
        let n = sock.recv(&mut bytes).unwrap();
        assert_eq!(&bytes[..n], b"READY=1");
    }
}
