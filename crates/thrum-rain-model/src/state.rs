//! The rain state as bytes, for saves and the wire: little-endian, versioned, and checked
//! against its tree when read back.
use crate::{Account, RainError, RainState, Tree};

const MAGIC: &[u8; 8] = b"THRRAIN1";

impl RainState {
    /// The state's bytes. Decoding them gives the same state, and encoding that the same
    /// bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + self.volume.len() * 9 + 56);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&self.tick.to_le_bytes());
        out.extend_from_slice(&(self.volume.len() as u64).to_le_bytes());
        for (&a, &v) in self.active.iter().zip(&self.volume) {
            out.push(u8::from(a));
            out.extend_from_slice(&v.to_le_bytes());
        }
        let a = &self.account;
        for v in [
            a.rain,
            a.canopy,
            a.ground,
            a.percolated,
            a.dried,
            a.edge,
            a.initial,
        ] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    /// Read a state back for a tree.
    /// # Errors
    /// Refuses bytes of another format or tree, a nonfinite or negative volume, and
    /// puddles that do not cover every hollow exactly once.
    pub fn decode(bytes: &[u8], tree: &Tree) -> Result<Self, RainError> {
        let mut r = Reader { bytes, at: 0 };
        if r.take(8)? != MAGIC {
            return Err(RainError::State("not rain state"));
        }
        let tick = r.u64()?;
        let n = usize::try_from(r.u64()?).map_err(|_| RainError::State("node count"))?;
        if n != tree.node_count() {
            return Err(RainError::State("state belongs to another tree"));
        }
        let mut active = Vec::with_capacity(n);
        let mut volume = Vec::with_capacity(n);
        for _ in 0..n {
            active.push(match r.take(1)?[0] {
                0 => false,
                1 => true,
                _ => return Err(RainError::State("invalid puddle flag")),
            });
            let v = r.f64()?;
            if !v.is_finite() || v < 0.0 {
                return Err(RainError::State("invalid volume"));
            }
            volume.push(v);
        }
        let mut f = || r.f64();
        let account = Account {
            rain: f()?,
            canopy: f()?,
            ground: f()?,
            percolated: f()?,
            dried: f()?,
            edge: f()?,
            initial: f()?,
        };
        if r.at != bytes.len() {
            return Err(RainError::State("trailing bytes"));
        }
        // Every hollow is covered by exactly one puddle on its path to its root.
        for h in 0..tree.hollow_count() {
            let mut k = Some(crate::fill::id(h));
            let mut count = 0;
            while let Some(node) = k {
                count += usize::from(active[node as usize]);
                k = tree.node(node).parent;
            }
            if count != 1 {
                return Err(RainError::State("puddles do not cover every hollow once"));
            }
        }
        Ok(Self {
            tick,
            volume,
            active,
            account,
        })
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], RainError> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.bytes.len());
        let end = end.ok_or(RainError::State("truncated"))?;
        let out = &self.bytes[self.at..end];
        self.at = end;
        Ok(out)
    }

    fn u64(&mut self) -> Result<u64, RainError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("eight bytes"),
        ))
    }

    fn f64(&mut self) -> Result<f64, RainError> {
        Ok(f64::from_le_bytes(
            self.take(8)?.try_into().expect("eight bytes"),
        ))
    }
}
