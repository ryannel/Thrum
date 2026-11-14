use std::{
    collections::VecDeque,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

/// Largest reliable message in bytes.
pub const RELIABLE_LIMIT: usize = 65_536;
/// Largest datagram in bytes, on every adapter.
pub const DATAGRAM_LIMIT: usize = 1_000;
/// Versioned startup frame, preceding all application bytes.
pub const HELLO: &[u8; 4] = b"THR1";
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Lane for a whole message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    /// Ordered, delivered once or closed.
    Reliable,
    /// Unordered, potentially lost.
    Datagram,
}
impl Lane {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Reliable => 0,
            Self::Datagram => 1,
        }
    }
    pub(crate) const fn limit(self) -> usize {
        match self {
            Self::Reliable => RELIABLE_LIMIT,
            Self::Datagram => DATAGRAM_LIMIT,
        }
    }
}

/// Congestion controller on Quinn ends; never selects a browser controller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Controller {
    /// Quinn's experimental BBR, measured beside Cubic.
    #[default]
    Bbr,
    /// Quinn's default loss-based controller.
    Cubic,
}
/// Per-link byte caps and timeouts. Both inbound and outbound lanes use these caps.
#[derive(Clone, Debug)]
pub struct Profile {
    /// Reliable bytes queued on each side of I/O, excluding at most one in-flight message.
    pub reliable_bytes: usize,
    /// Datagram bytes queued on each side of I/O.
    pub datagram_bytes: usize,
    /// Datagram wait in the link queue, in milliseconds.
    pub age_ms: u64,
    /// Datagram bytes held beneath the link queue, without an arrival deadline.
    pub stack_bytes: usize,
    /// Startup deadline, in milliseconds.
    pub startup_ms: u64,
    /// Idle timeout, in milliseconds.
    pub idle_ms: u64,
    /// Maximum simultaneous links, including handshakes.
    pub links: usize,
    /// Controller used on native ends.
    pub controller: Controller,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            reliable_bytes: 1_048_576,
            datagram_bytes: 64_000,
            age_ms: 250,
            stack_bytes: 4_096,
            startup_ms: 4_000,
            idle_ms: 4_000,
            links: 4,
            controller: Controller::Bbr,
        }
    }
}
impl Profile {
    /// Validate a profile before creating any link or host.
    /// # Errors
    /// Returns `InvalidProfile` for caps below a message or zero timeouts/capacity.
    pub fn validate(&self) -> Result<(), LinkError> {
        if self.reliable_bytes < RELIABLE_LIMIT
            || self.datagram_bytes < DATAGRAM_LIMIT
            || self.stack_bytes < DATAGRAM_LIMIT
            || self.age_ms == 0
            || self.startup_ms == 0
            || self.idle_ms == 0
            || self.links == 0
        {
            return Err(LinkError::InvalidProfile);
        }
        Ok(())
    }
}
/// Why a send or construction was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkError {
    /// A zero-length message is not permitted.
    Empty,
    /// A message exceeds its lane's limit.
    TooLarge,
    /// Reliable queue has no room; retry after it drains.
    Full,
    /// The link has not opened or has closed.
    NotOpen,
    /// Profile caps or timeouts are invalid.
    InvalidProfile,
    /// Ticket text or version is invalid.
    InvalidTicket,
    /// The adapter could not start its local resources.
    StartFailed,
}
impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for LinkError {}
/// Terminal reason. Closed links never deliver another message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloseReason {
    /// Closed by this game.
    Local(u32),
    /// Closed by the remote game, with its code.
    Peer(u32),
    /// No startup or liveness progress by the deadline.
    TimedOut,
    /// Identity, version or host capacity refused the connection.
    Refused,
    /// Peer violated framing, stream count or message limit.
    Protocol,
    /// The underlying transport failed.
    Transport,
}
/// Monotonic lifecycle event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Startup completed without a game send.
    Open,
    /// Terminal; no queued inbound messages survive it.
    Closed(CloseReason),
}
/// A lane's application counts. Sent counts only accepted sends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LaneCounts {
    /// Accepted messages.
    pub sent: u64,
    /// Accepted bytes.
    pub sent_bytes: u64,
    /// Messages delivered to the game.
    pub received: u64,
    /// Bytes delivered to the game.
    pub received_bytes: u64,
    /// Current outbound bytes.
    pub queued_bytes: usize,
    /// Oldest outbound age in milliseconds, absent for an empty queue.
    pub oldest_ms: Option<u64>,
}
/// Snapshot of bounded queue and delivery accounting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    /// Reliable, then datagram lane.
    pub lanes: [LaneCounts; 2],
    /// Empty, too large, full, not open refusals.
    pub refused: [u64; 4],
    /// Datagrams removed from our queue and handed to the stack.
    pub handed_on: u64,
    /// Datagrams expired in our queue, never inferred from packet loss.
    pub expired: u64,
    /// Outbound datagrams discarded oldest-first at capacity.
    pub dropped_cap: u64,
    /// Inbound datagrams discarded oldest-first at capacity.
    pub dropped_inbound: u64,
    /// Stack buffer bytes charged, including library metadata; absent when not reported.
    pub stack_bytes: Option<usize>,
    /// Transport round-trip estimate in microseconds.
    pub rtt_us: Option<u64>,
    /// Transport packets sent.
    pub packets_sent: Option<u64>,
    /// Transport packets declared lost (including reordering).
    pub packets_lost: Option<u64>,
    /// UDP bytes sent by this endpoint.
    pub wire_bytes: Option<u64>,
}
#[derive(Clone, Debug)]
pub(crate) struct Message {
    pub at: u64,
    pub bytes: Vec<u8>,
}
#[derive(Debug)]
pub(crate) struct State {
    pub clock: Option<std::time::Instant>,
    pub clock_fn: Option<fn() -> u64>,
    pub profile: Profile,
    pub open: bool,
    pub closed: Option<CloseReason>,
    pub events: VecDeque<Event>,
    pub outbound: [VecDeque<Message>; 2],
    pub inbound: [VecDeque<Vec<u8>>; 2],
    pub inbound_bytes: [usize; 2],
    pub counts: Counters,
}
impl State {
    fn time(&self, caller: u64) -> u64 {
        if let Some(clock) = self.clock_fn {
            return clock();
        }
        self.clock.map_or(caller, |clock| {
            u64::try_from(clock.elapsed().as_millis()).unwrap_or(u64::MAX)
        })
    }
    pub(crate) fn expire(&mut self, now: u64) {
        while self.outbound[1]
            .front()
            .is_some_and(|m| now.saturating_sub(m.at) >= self.profile.age_ms)
        {
            let message = self.outbound[1].pop_front().expect("front exists");
            self.counts.lanes[1].queued_bytes -= message.bytes.len();
            self.counts.expired += 1;
        }
    }
    pub(crate) fn opened(&mut self) {
        if self.closed.is_none() && !self.open {
            self.open = true;
            self.events.push_back(Event::Open);
        }
    }
    pub(crate) fn close(&mut self, reason: CloseReason) {
        if self.closed.is_some() {
            return;
        }
        self.open = false;
        self.closed = Some(reason.clone());
        self.events.push_back(Event::Closed(reason));
        for queue in &mut self.outbound {
            queue.clear();
        }
        for queue in &mut self.inbound {
            queue.clear();
        }
        self.inbound_bytes = [0; 2];
        for lane in &mut self.counts.lanes {
            lane.queued_bytes = 0;
        }
    }
    pub(crate) fn pop(&mut self, lane: Lane) -> Option<Vec<u8>> {
        let message = self.outbound[lane.index()].pop_front()?;
        self.counts.lanes[lane.index()].queued_bytes -= message.bytes.len();
        if lane == Lane::Datagram {
            self.counts.handed_on += 1;
        }
        Some(message.bytes)
    }
    pub(crate) fn receive(&mut self, lane: Lane, bytes: Vec<u8>) -> bool {
        if !self.open {
            return false;
        }
        let index = lane.index();
        let cap = if index == 0 {
            self.profile.reliable_bytes
        } else {
            self.profile.datagram_bytes
        };
        let mut used = self.inbound_bytes[index];
        if index == 0 && used + bytes.len() > cap {
            return false;
        }
        while used + bytes.len() > cap {
            used -= self.inbound[index]
                .pop_front()
                .expect("bounded nonempty queue")
                .len();
            self.counts.dropped_inbound += 1;
        }
        self.inbound_bytes[index] = used + bytes.len();
        self.inbound[index].push_back(bytes);
        true
    }
}

/// A game's nonblocking handle. Clones refer to the same link, never a new connection.
#[derive(Clone, Debug)]
pub struct Link {
    id: u64,
    pub(crate) state: Arc<Mutex<State>>,
}
impl Link {
    pub(crate) fn new(profile: Profile) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            state: Arc::new(Mutex::new(State {
                clock: None,
                clock_fn: None,
                profile,
                open: false,
                closed: None,
                events: VecDeque::new(),
                outbound: std::array::from_fn(|_| VecDeque::new()),
                inbound: std::array::from_fn(|_| VecDeque::new()),
                inbound_bytes: [0; 2],
                counts: Counters::default(),
            })),
        }
    }
    /// Process-unique link identity, retained after close.
    pub const fn id(&self) -> u64 {
        self.id
    }
    /// Whether startup has completed and no close has occurred.
    /// # Panics
    /// Panics if an internal I/O panic poisoned the link state.
    pub fn is_open(&self) -> bool {
        self.state.lock().expect("link state poisoned").open
    }
    /// Queue a whole message, copying it once. `now` supplies virtual milliseconds for the
    /// in-memory carrier; socket adapters measure queue age on their own monotonic clock.
    /// # Errors
    /// Refuses empty, oversize, non-open sends and reliable sends beyond the queue cap.
    /// # Panics
    /// Panics if an internal I/O panic poisoned the link state.
    pub fn send(&self, lane: Lane, bytes: &[u8], now: u64) -> Result<(), LinkError> {
        let mut state = self.state.lock().expect("link state poisoned");
        let now = state.time(now);
        let index = lane.index();
        let refused = if bytes.is_empty() {
            Some((LinkError::Empty, 0))
        } else if bytes.len() > lane.limit() {
            Some((LinkError::TooLarge, 1))
        } else if !state.open {
            Some((LinkError::NotOpen, 3))
        } else if index == 0
            && state.counts.lanes[0].queued_bytes + bytes.len() > state.profile.reliable_bytes
        {
            Some((LinkError::Full, 2))
        } else {
            None
        };
        if let Some((error, counter)) = refused {
            state.counts.refused[counter] += 1;
            return Err(error);
        }
        state.expire(now);
        if index == 1 {
            while state.counts.lanes[1].queued_bytes + bytes.len() > state.profile.datagram_bytes {
                let old = state.outbound[1].pop_front().expect("queue at capacity");
                state.counts.lanes[1].queued_bytes -= old.bytes.len();
                state.counts.dropped_cap += 1;
            }
        }
        state.outbound[index].push_back(Message {
            at: now,
            bytes: bytes.to_vec(),
        });
        state.counts.lanes[index].queued_bytes += bytes.len();
        state.counts.lanes[index].sent += 1;
        state.counts.lanes[index].sent_bytes += bytes.len() as u64;
        Ok(())
    }
    /// Take the next arrived whole message. This never reads a socket.
    /// # Panics
    /// Panics if an internal I/O panic poisoned the link state.
    pub fn receive(&self, lane: Lane) -> Option<Vec<u8>> {
        let mut state = self.state.lock().expect("link state poisoned");
        let bytes = state.inbound[lane.index()].pop_front()?;
        state.inbound_bytes[lane.index()] -= bytes.len();
        state.counts.lanes[lane.index()].received += 1;
        state.counts.lanes[lane.index()].received_bytes += bytes.len() as u64;
        Some(bytes)
    }
    /// Take the next lifecycle event; at most open and closed are queued.
    /// # Panics
    /// Panics if an internal I/O panic poisoned the link state.
    pub fn event(&self) -> Option<Event> {
        self.state
            .lock()
            .expect("link state poisoned")
            .events
            .pop_front()
    }
    /// Close immediately. Queued messages are discarded; delivery before close is a prefix.
    /// # Panics
    /// Panics if an internal I/O panic poisoned the link state.
    pub fn close(&self, code: u32) {
        self.state
            .lock()
            .expect("link state poisoned")
            .close(CloseReason::Local(code));
    }
    /// Take a counter snapshot. Timers are driven by I/O, or by `Memory::advance`.
    /// # Panics
    /// Panics if an internal I/O panic poisoned the link state.
    pub fn counters(&self, now: u64) -> Counters {
        let state = self.state.lock().expect("link state poisoned");
        let now = state.time(now);
        let mut result = state.counts.clone();
        for (index, queue) in state.outbound.iter().enumerate() {
            result.lanes[index].oldest_ms = queue.front().map(|m| now.saturating_sub(m.at));
        }
        result
    }
}

/// Address, version and SHA-256 certificate fingerprint. Contains no secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket {
    /// Host address, including port, accepted by the selected adapter.
    pub address: String,
    /// SHA-256 of the DER certificate.
    pub hash: [u8; 32],
}
impl fmt::Display for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "thrum1@{}@", self.address)?;
        for byte in self.hash {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
impl std::str::FromStr for Ticket {
    type Err = LinkError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() > 320 {
            return Err(LinkError::InvalidTicket);
        }
        let parts: Vec<_> = text.split('@').collect();
        if parts.len() != 3
            || parts[0] != "thrum1"
            || parts[1].is_empty()
            || parts[2].len() != 64
            || !parts[2].is_ascii()
        {
            return Err(LinkError::InvalidTicket);
        }
        let mut hash = [0; 32];
        for (index, byte) in hash.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&parts[2][index * 2..index * 2 + 2], 16)
                .map_err(|_| LinkError::InvalidTicket)?;
        }
        Ok(Self {
            address: parts[1].into(),
            hash,
        })
    }
}
