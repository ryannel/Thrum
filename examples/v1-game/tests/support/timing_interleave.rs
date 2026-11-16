//! Cost-only coordination between two frozen shader builds; no renderer changes.
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

pub(super) struct TimingInterleave {
    stream: TcpStream,
    arm: u8,
    row: u64,
}

impl TimingInterleave {
    pub(super) fn from_env() -> Option<Self> {
        let Some(address) = std::env::var_os("THRUM_CULLING_TIMING_PAIR") else {
            assert!(std::env::var_os("THRUM_CULLING_TIMING_ARM").is_none());
            return None;
        };
        let arm: u8 = std::env::var("THRUM_CULLING_TIMING_ARM")
            .expect("paired timing arm")
            .parse()
            .expect("numeric paired timing arm");
        assert!(arm <= 1, "two timing arms");
        let address: SocketAddr = address
            .to_str()
            .expect("timing address")
            .parse()
            .expect("numeric timing socket address");
        assert!(address.ip().is_loopback(), "loopback timing coordinator");
        let mut stream = TcpStream::connect(address).expect("timing coordinator");
        stream.set_nodelay(true).expect("timing TCP no delay");
        let timeout = Some(Duration::from_secs(120));
        stream
            .set_read_timeout(timeout)
            .expect("timing read timeout");
        stream
            .set_write_timeout(timeout)
            .expect("timing write timeout");
        stream
            .write_all(&[1, arm])
            .expect("timing protocol and arm");
        Some(Self {
            stream,
            arm,
            row: 0,
        })
    }

    pub(super) fn begin(&mut self) -> serde_json::Value {
        self.stream
            .write_all(&self.row.to_le_bytes())
            .expect("ready timing row");
        let mut grant = [0];
        self.stream
            .read_exact(&mut grant)
            .expect("timing row grant");
        assert_eq!(grant, [1], "valid timing row grant");
        serde_json::json!({"arm": self.arm, "row": self.row})
    }

    pub(super) fn completed(&mut self) {
        // Called only after FrameSample's GPU completion. The peer may then
        // submit its matching row; neither host changes its fixed route clock.
        self.stream.write_all(&[1]).expect("completed timing row");
        self.row += 1;
    }
}
