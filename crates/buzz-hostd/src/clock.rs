use anyhow::Result;
use rustix::time::{clock_gettime, ClockId};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct Clock {
    pub source: Option<fn() -> Result<Self>>,
    pub wall: u64,
    pub boot: String,
    pub millis: u64,
}

impl Clock {
    pub fn refresh(&self) -> Result<Self> {
        self.source.map_or_else(|| Ok(self.clone()), |f| f())
    }
    pub fn read() -> Result<Self> {
        let mono = clock_gettime(ClockId::Boottime);
        Ok(Self {
            source: Some(Self::read),
            wall: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            boot: fs::read_to_string("/proc/sys/kernel/random/boot_id")?
                .trim()
                .into(),
            millis: mono.tv_sec as u64 * 1000 + mono.tv_nsec as u64 / 1000000,
        })
    }
}
