//! Candidate exact transformed local-box admission predicate.
//!
//! The caller supplies the tight componentwise extrema of *all* decoded vertices,
//! including unreferenced vertices; an empty mesh supplies the origin point. The
//! caller must separately prove each vertex's exact squared radius is <= 10^6.
//! Merely checking each box coordinate is within +/-1000 is insufficient to prove
//! that radius rule (box corners themselves need not meet it).
//!
//! The interval graph is retained from R0-F3-TRANSFORMED-BOUND-OPT-2026-11-03.
//! Each operation takes one outward binary64 step. This requires IEEE binary64
//! round-to-nearest with gradual underflow, without fast-math reassociation; it
//! is not a compiler/device qualification. Uncertain cases use exact dyadics.
//!
//! Fixed capacity: with D=2^1074, finite components are X/D, |X|<2^2098.
//! Unrestricted quaternion norm checking needs <4258 bits. After the exact norm
//! passes, |q|<2 and QI<2^2150. For local coordinates <=1000, translation <=10^6
//! and unrestricted finite scale, each NI*sI*pI is <2^5332 and complete support
//! sums, including intermediates, are <2^5335. 84 limbs give 5376 bits. The
//! normalized representation removes only powers of two, so alignment cannot
//! exceed that fixed-denominator bound. The bounded four-factor graph keeps
//! exponents within +/-10000. Extending this arithmetic needs a new proof.
//!
//! This pure predicate retains no cache and allocates no heap storage. It does
//! not establish integrated F3 latency, charged work, memory or F4 support.

const L: usize = 84;
const WORLD_EXTENT: f64 = 1_000_000.0;
const LOCAL_COORDINATE_EXTENT: f64 = 1000.0;

/// Invalid decoded input is distinct from a finite world-extent refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransformSourceError {
    InvalidSource,
    InvalidQuaternion,
}

/// Checks the closed affine image of the supplied local box against +/-10^6 m.
///
/// `Ok(false)` is an F3 world-extent source refusal, never F4 `Unsupported`.
/// Signed/zero scales and either quaternion sign retain their exact meaning.
/// Finite translations outside the authored world interval refuse even when
/// the transformed box would be moved back inside. Invalid quaternions remain
/// errors even for an otherwise out-of-world finite translation.
pub(crate) fn transformed_box_within_world(
    local_min: [f64; 3],
    local_max: [f64; 3],
    translation: [f64; 3],
    orientation_xyzw: [f64; 4],
    scale: [f64; 3],
) -> Result<bool, TransformSourceError> {
    if !local_min
        .iter()
        .chain(&local_max)
        .chain(&translation)
        .chain(&orientation_xyzw)
        .chain(&scale)
        .all(|v| v.is_finite())
        || (0..3).any(|j| {
            local_min[j] > local_max[j]
                || local_min[j].abs() > LOCAL_COORDINATE_EXTENT
                || local_max[j].abs() > LOCAL_COORDINATE_EXTENT
        })
    {
        return Err(TransformSourceError::InvalidSource);
    }
    let c = Case {
        q: orientation_xyzw,
        t: translation,
        s: scale,
        lo: local_min,
        hi: local_max,
    };
    // Preserve the arithmetic width precondition before constructing supports.
    if translation.iter().any(|v| v.abs() > WORLD_EXTENT) {
        let (valid, _, _) = q_exact(c.q);
        return if valid {
            Ok(false)
        } else {
            Err(TransformSourceError::InvalidQuaternion)
        };
    }
    let (valid, inside) = filter(c).unwrap_or_else(|| exact(c));
    if valid {
        Ok(inside)
    } else {
        Err(TransformSourceError::InvalidQuaternion)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Dyadic {
    neg: bool,
    a: [u64; L],
    used: usize,
    exp: i32,
}
impl Dyadic {
    fn zero() -> Self {
        Self {
            neg: false,
            a: [0; L],
            used: 0,
            exp: 0,
        }
    }
    fn small(x: u64) -> Self {
        let mut r = Self::zero();
        if x != 0 {
            r.a[0] = x;
            r.used = 1;
        }
        r.norm();
        r
    }
    fn norm(&mut self) {
        while self.used > 0 && self.a[self.used - 1] == 0 {
            self.used -= 1;
        }
        if self.used == 0 {
            self.neg = false;
            self.exp = 0;
            return;
        }
        let low = self.a[..self.used].iter().position(|x| *x != 0).unwrap();
        let shift = self.a[low].trailing_zeros();
        let old_used = self.used;
        for i in 0..old_used - low {
            let j = i + low;
            self.a[i] = self.a[j] >> shift;
            if shift != 0 && j + 1 < old_used {
                self.a[i] |= self.a[j + 1] << (64 - shift);
            }
        }
        self.used -= low;
        self.exp = self
            .exp
            .checked_add((low * 64) as i32 + shift as i32)
            .expect("bounded dyadic exponent");
        while self.used > 0 && self.a[self.used - 1] == 0 {
            self.used -= 1;
        }
        self.a[self.used..old_used].fill(0);
    }
    fn aligned(&self, exp: i32) -> Self {
        if self.used == 0 {
            return Self::zero();
        }
        let shift =
            usize::try_from(self.exp.checked_sub(exp).expect("bounded dyadic alignment")).unwrap();
        let mut r = Self::zero();
        r.exp = exp;
        r.neg = self.neg;
        for i in 0..self.used {
            let j = i + shift / 64;
            assert!(j < L);
            r.a[j] |= self.a[i] << (shift % 64);
            r.used = r.used.max(j + 1);
            if shift % 64 != 0 && self.a[i] >> (64 - shift % 64) != 0 {
                assert!(j + 1 < L);
                r.a[j + 1] |= self.a[i] >> (64 - shift % 64);
                r.used = r.used.max(j + 2);
            }
        }
        r
    }
    fn cmpabs(&self, o: &Self) -> std::cmp::Ordering {
        if self.used == 0 || o.used == 0 {
            return self.used.cmp(&o.used);
        }
        let bit = self.exp + (self.used * 64) as i32 - self.a[self.used - 1].leading_zeros() as i32;
        let other = o.exp + (o.used * 64) as i32 - o.a[o.used - 1].leading_zeros() as i32;
        if bit != other {
            return bit.cmp(&other);
        }
        if self.exp == o.exp {
            return self.a[..self.used]
                .iter()
                .rev()
                .cmp(o.a[..o.used].iter().rev());
        }
        let exp = self.exp.min(o.exp);
        let a = self.aligned(exp);
        let b = o.aligned(exp);
        a.a[..a.used].iter().rev().cmp(b.a[..b.used].iter().rev())
    }
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        if self.neg != o.neg {
            return if self.neg {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        let c = self.cmpabs(o);
        if self.neg { c.reverse() } else { c }
    }
    fn minus(mut self) -> Self {
        if self.used != 0 {
            self.neg = !self.neg;
        }
        self
    }
    fn shl(&self, k: usize) -> Self {
        let mut r = self.clone();
        if r.used != 0 {
            r.exp = r
                .exp
                .checked_add(i32::try_from(k).expect("bounded shift"))
                .expect("bounded dyadic exponent");
        }
        r
    }
    fn add(&self, o: &Self) -> Self {
        if self.used == 0 {
            return o.clone();
        }
        if o.used == 0 {
            return self.clone();
        }
        let exp = self.exp.min(o.exp);
        let a = self.aligned(exp);
        let b = o.aligned(exp);
        let mut r = Self::zero();
        r.exp = exp;
        let n = a.used.max(b.used);
        if a.neg == b.neg {
            r.neg = a.neg;
            let mut c = 0u128;
            for i in 0..n {
                let t = a.a[i] as u128 + b.a[i] as u128 + c;
                r.a[i] = t as u64;
                c = t >> 64;
            }
            r.used = n;
            if c != 0 {
                assert!(n < L);
                r.a[n] = c as u64;
                r.used += 1;
            }
        } else {
            let (a, b) = if a.a[..n].iter().rev().cmp(b.a[..n].iter().rev()).is_lt() {
                (&b, &a)
            } else {
                (&a, &b)
            };
            r.neg = a.neg;
            r.used = n;
            let mut borrow = false;
            for i in 0..n {
                let (x, b1) = a.a[i].overflowing_sub(b.a[i]);
                let (y, b2) = x.overflowing_sub(borrow as u64);
                r.a[i] = y;
                borrow = b1 || b2;
            }
            assert!(!borrow);
        }
        r.norm();
        r
    }
    fn sub(&self, o: &Self) -> Self {
        self.add(&o.clone().minus())
    }
    fn mul(&self, o: &Self) -> Self {
        if self.used == 0 || o.used == 0 {
            return Self::zero();
        }
        let mut r = Self::zero();
        r.exp = self
            .exp
            .checked_add(o.exp)
            .expect("bounded dyadic exponent");
        for i in 0..self.used {
            if self.a[i] == 0 {
                continue;
            }
            let mut c = 0u128;
            for j in 0..o.used {
                let k = i + j;
                assert!(k < L);
                let t = self.a[i] as u128 * o.a[j] as u128 + r.a[k] as u128 + c;
                r.a[k] = t as u64;
                c = t >> 64;
            }
            let k = i + o.used;
            if c != 0 {
                assert!(k < L);
                assert_eq!(r.a[k], 0);
                r.a[k] = c as u64;
            }
        }
        r.used = (self.used + o.used).min(L);
        r.neg = self.neg != o.neg;
        r.norm();
        r
    }
    fn f64(x: f64) -> Self {
        assert!(x.is_finite());
        let bits = x.to_bits();
        let e = ((bits >> 52) & 2047) as i32;
        let m = (bits & ((1u64 << 52) - 1)) | if e == 0 { 0 } else { 1u64 << 52 };
        let mut r = Self::small(m);
        if r.used != 0 {
            r.exp = r
                .exp
                .checked_add(if e == 0 { -1074 } else { e - 1075 })
                .expect("decoded binary64 exponent");
            r.neg = x.is_sign_negative();
        }
        r
    }
}
#[derive(Clone, Copy, Debug)]
struct Interval {
    lo: f64,
    hi: f64,
}
fn down(x: f64) -> f64 {
    if x.is_nan() || x == f64::NEG_INFINITY {
        x
    } else if x == 0.0 {
        -f64::from_bits(1)
    } else {
        f64::from_bits(if x > 0.0 {
            x.to_bits() - 1
        } else {
            x.to_bits() + 1
        })
    }
}
fn up(x: f64) -> f64 {
    -down(-x)
}
impl Interval {
    fn p(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn all() -> Self {
        Self {
            lo: f64::NEG_INFINITY,
            hi: f64::INFINITY,
        }
    }
    fn add(self, o: Self) -> Self {
        let a = self.lo + o.lo;
        let b = self.hi + o.hi;
        if a.is_nan() || b.is_nan() {
            Self::all()
        } else {
            Self {
                lo: down(a),
                hi: up(b),
            }
        }
    }
    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    fn sub(self, o: Self) -> Self {
        self.add(o.neg())
    }
    fn mul(self, o: Self) -> Self {
        let v = [
            self.lo * o.lo,
            self.lo * o.hi,
            self.hi * o.lo,
            self.hi * o.hi,
        ];
        if v.iter().any(|x| x.is_nan()) {
            return Self::all();
        }
        Self {
            lo: down(v.iter().copied().fold(f64::INFINITY, f64::min)),
            hi: up(v.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
        }
    }
    fn div(self, o: Self) -> Self {
        if o.lo <= 0.0 {
            return Self::all();
        }
        self.mul(Self {
            lo: down(1.0 / o.hi),
            hi: up(1.0 / o.lo),
        })
    }
}
fn q_exact(q: [f64; 4]) -> (bool, Dyadic, [Dyadic; 9]) {
    let [x, y, z, w] = q.map(Dyadic::f64);
    let xx = x.mul(&x);
    let yy = y.mul(&y);
    let zz = z.mul(&z);
    let ww = w.mul(&w);
    let norm = xx.add(&yy).add(&zz).add(&ww);
    let lo = Dyadic::small(999999999).mul(&Dyadic::small(999999999));
    let hi = Dyadic::small(1000000001).mul(&Dyadic::small(1000000001));
    let test = norm.mul(&Dyadic::small(1_000_000_000_000_000_000));
    let valid = !test.cmp(&lo).is_lt() && !test.cmp(&hi).is_gt();
    if !valid {
        return (false, norm, std::array::from_fn(|_| Dyadic::zero()));
    }
    let n = [
        ww.add(&xx).sub(&yy).sub(&zz),
        x.mul(&y).sub(&z.mul(&w)).shl(1),
        x.mul(&z).add(&y.mul(&w)).shl(1),
        x.mul(&y).add(&z.mul(&w)).shl(1),
        ww.sub(&xx).add(&yy).sub(&zz),
        y.mul(&z).sub(&x.mul(&w)).shl(1),
        x.mul(&z).sub(&y.mul(&w)).shl(1),
        y.mul(&z).add(&x.mul(&w)).shl(1),
        ww.sub(&xx).sub(&yy).add(&zz),
    ];
    (true, norm, n)
}
fn q_interval(q: [f64; 4]) -> (Option<bool>, Interval, [Interval; 9]) {
    let [x, y, z, w] = q.map(Interval::p);
    let xx = x.mul(x);
    let yy = y.mul(y);
    let zz = z.mul(z);
    let ww = w.mul(w);
    let norm = xx.add(yy).add(zz).add(ww);
    let eps = Interval::p(1.0).div(Interval::p(1e9));
    let lo = Interval::p(1.0).sub(eps);
    let lo = lo.mul(lo);
    let hi = Interval::p(1.0).add(eps);
    let hi = hi.mul(hi);
    let v = if norm.hi < lo.lo || norm.lo > hi.hi {
        Some(false)
    } else if norm.lo >= lo.hi && norm.hi <= hi.lo {
        Some(true)
    } else {
        None
    };
    let two = Interval::p(2.0);
    let n = [
        ww.add(xx).sub(yy).sub(zz),
        x.mul(y).sub(z.mul(w)).mul(two),
        x.mul(z).add(y.mul(w)).mul(two),
        x.mul(y).add(z.mul(w)).mul(two),
        ww.sub(xx).add(yy).sub(zz),
        y.mul(z).sub(x.mul(w)).mul(two),
        x.mul(z).sub(y.mul(w)).mul(two),
        y.mul(z).add(x.mul(w)).mul(two),
        ww.sub(xx).sub(yy).add(zz),
    ];
    (v, norm, n)
}
#[derive(Clone, Copy)]
struct Case {
    q: [f64; 4],
    t: [f64; 3],
    s: [f64; 3],
    lo: [f64; 3],
    hi: [f64; 3],
}
fn exact(c: Case) -> (bool, bool) {
    let (v, q, n) = q_exact(c.q);
    if !v {
        return (false, false);
    }
    let limit = q.mul(&Dyadic::small(1_000_000));
    let t = c.t.map(Dyadic::f64);
    let s = c.s.map(Dyadic::f64);
    let lo = c.lo.map(Dyadic::f64);
    let hi = c.hi.map(Dyadic::f64);
    for a in 0..3 {
        let center = t[a].mul(&q);
        let mut mn = center.clone();
        let mut mx = center;
        for j in 0..3 {
            let coef = n[a * 3 + j].mul(&s[j]);
            let (l, h) = if coef.neg {
                (&hi[j], &lo[j])
            } else {
                (&lo[j], &hi[j])
            };
            mn = mn.add(&coef.mul(l));
            mx = mx.add(&coef.mul(h))
        }
        if mn.cmp(&limit.clone().minus()).is_lt() || mx.cmp(&limit).is_gt() {
            return (true, false);
        }
    }
    (true, true)
}
fn filter(c: Case) -> Option<(bool, bool)> {
    let (v, q, n) = q_interval(c.q);
    if v == Some(false) {
        return Some((false, false));
    }
    if v != Some(true) {
        return None;
    }
    let mut certain = true;
    for a in 0..3 {
        let mut mn = Interval::p(c.t[a]);
        let mut mx = mn;
        for j in 0..3 {
            let coef = n[a * 3 + j].div(q).mul(Interval::p(c.s[j]));
            let p = Interval {
                lo: c.lo[j],
                hi: c.hi[j],
            };
            let term = coef.mul(p);
            mn = mn.add(Interval::p(term.lo));
            mx = mx.add(Interval::p(term.hi))
        }
        if mx.hi < -1e6 || mn.lo > 1e6 {
            return Some((true, false));
        }
        if mn.lo < -1e6 || mx.hi > 1e6 {
            certain = false
        }
    }
    if certain { Some((true, true)) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(c: Case) -> Result<bool, TransformSourceError> {
        transformed_box_within_world(c.lo, c.hi, c.t, c.q, c.s)
    }

    fn unit_box() -> Case {
        Case {
            q: [0.0, 0.0, 0.0, 1.0],
            t: [0.0; 3],
            s: [1.0; 3],
            lo: [-1.0; 3],
            hi: [1.0; 3],
        }
    }

    // Decoded bit patterns and expected outcomes from the independent Python
    // Fraction eight-corner oracle in the retained PROBE and OPT archives.
    // These are selected fixed vectors, not expectations computed by our support
    // selection or Dyadic implementation. The complete archived corpus is wider.
    const ORACLE_VECTORS: &str = r#"identity 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
q_neg_identity 0000000000000000 8000000000000000 0000000000000000 bff0000000000000 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
upper_equal 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 412e847e00000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
upper_next_in 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 412e847dffffffff 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
upper_next_out 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 412e847e00000001 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 0
lower_equal 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 c12e847e00000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
lower_next_in 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 c12e847dffffffff 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
lower_next_out 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 c12e847e00000001 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 0
q_lower_below 0000000000000000 0000000000000000 0000000000000000 3fefffffff768fa0 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 0 0
q_lower_above 0000000000000000 0000000000000000 0000000000000000 3fefffffff768fa1 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
q_upper_below 0000000000000000 0000000000000000 0000000000000000 3ff000000044b82f 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
q_upper_above 0000000000000000 0000000000000000 0000000000000000 3ff000000044b830 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 0 0
q_zero 0000000000000000 0000000000000000 0000000000000000 0000000000000000 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 0 0
q_subnormal 0000000000000001 0000000000000000 0000000000000000 0000000000000000 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 0 0
q_max_finite 7fefffffffffffff 7fefffffffffffff 7fefffffffffffff 7fefffffffffffff 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 0 0
signed_scale 0000000000000000 0000000000000000 3fe3333333333333 3fe999999999999a 0000000000000000 0000000000000000 0000000000000000 c000000000000000 8000000000000000 bff0000000000000 c008000000000000 c01c000000000000 0000000000000000 4010000000000000 4020000000000000 0000000000000000 1 1
zero_scale_at_extent 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 412e848000000000 c12e848000000000 412e848000000000 0000000000000000 8000000000000000 0000000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
empty_mesh_origin 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 412e848000000000 c12e848000000000 412e848000000000 7fefffffffffffff 7fefffffffffffff 7fefffffffffffff 0000000000000000 0000000000000000 0000000000000000 0000000000000000 0000000000000000 0000000000000000 1 1
subnormal_support_inside 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 8000000000000001 8000000000000001 8000000000000001 0000000000000001 0000000000000001 0000000000000001 1 1
subnormal_support_outside 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 412e848000000000 0000000000000000 0000000000000000 0000000000000001 3ff0000000000000 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 0000000000000001 0000000000000000 0000000000000000 1 0
huge_scale_subnormal_point 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 7fefffffffffffff 7fefffffffffffff 7fefffffffffffff 0000000000000001 0000000000000001 0000000000000001 0000000000000001 0000000000000001 0000000000000001 1 1
huge_scale_subnormal_rotation 0000000000000001 0000000000000000 0000000000000000 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 7fefffffffffffff 7fefffffffffffff 7fefffffffffffff 0000000000000000 0000000000000001 0000000000000000 0000000000000000 0000000000000001 0000000000000000 1 1
huge_scale_refusal 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 7fefffffffffffff 7fefffffffffffff 7fefffffffffffff bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 0
rotated_empty_corner_refusal 0000000000000000 0000000000000000 3fe3333333333333 3fe999999999999a 0000000000000000 412e7cb000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 c08f400000000000 0000000000000000 0000000000000000 408f400000000000 408f400000000000 0000000000000000 1 0
unreferenced_outlier 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 412e809800000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 408f400000000000 3ff0000000000000 0000000000000000 1 0
wide_span_refusal 0000000000000001 0000000000000001 0000000000000001 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 7fefffffffffffff 7fefffffffffffff 7fefffffffffffff c08f400000000000 c08f400000000000 c08f400000000000 408f400000000000 408f400000000000 408f400000000000 1 0
subnormal_cancellation 0000000000000001 0000000000000001 0000000000000001 3ff0000000000000 412e848000000000 412e848000000000 412e848000000000 7fefffffffffffff 7fefffffffffffff 7fefffffffffffff 8000000000000001 8000000000000001 8000000000000001 8000000000000001 8000000000000001 8000000000000001 1 1
scaled_q_equivalent 3fe0000000000001 3fe0000000000001 3fe0000000000001 3fe0000000000001 412e847e00000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
base_q_equivalent 3fe0000000000000 3fe0000000000000 3fe0000000000000 3fe0000000000000 412e847e00000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 bff0000000000000 bff0000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 3ff0000000000000 1 1
negative_scale_boundary 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 412e848000000000 0000000000000000 0000000000000000 bff0000000000000 3ff0000000000000 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 0000000000000000 0000000000000000 1 1
mixed_70 0000000000000000 0000000000000000 3fe3333333333333 3fe999999999999a 412e7cb000000000 412e7cb000000000 412e7cb000000000 0000000000000000 8000000000000001 7fefffffffffffff 8000000000000000 8000000000000001 20b0000000000000 0000000000000000 3ff0000000000000 20b0000000000000 1 0
mixed_170 0000000000000000 0000000000000000 0000000000000000 bff0000000000000 412e847e00000001 c12e847e00000000 412e848000000000 8000000000000001 0000000000000001 7fefffffffffffff bff0000000000000 0000000000000001 8000000000000001 408f400000000000 408f400000000000 8000000000000001 1 1
mixed_370 0000000000000000 0000000000000000 3fe3333333333333 3fe999999999999a 412e847e00000001 412e7cb000000000 412e848000000000 7fefffffffffffff 408f400000000000 bff0000000000000 bff0000000000000 3ff0000000000000 20b0000000000000 20b0000000000000 408f400000000000 3fe0000000000000 1 0
mixed_670 3fe0000000000000 3fe0000000000000 3fe0000000000000 3fe0000000000000 412e7cb000000000 0000000000000000 412e847e00000001 7fefffffffffffff 7fefffffffffffff 8000000000000001 8000000000000001 8000000000000001 bff0000000000000 408f400000000000 0000000000000001 bfe0000000000000 1 0
mixed_970 3fe0000000000000 3fe0000000000000 3fe0000000000000 3fe0000000000000 c12e847e00000000 c12e847e00000000 412e7cb000000000 0170000000000000 3ff0000000000000 8000000000000000 8000000000000000 bff0000000000000 8000000000000000 20b0000000000000 3ff0000000000000 20b0000000000000 1 1
mixed_1370 0000000000000000 0000000000000000 0000000000000000 bff0000000000000 412e7cb000000000 c12e848000000000 c12e847e00000000 3ff0000000000000 5f30000000000000 5f30000000000000 0000000000000001 8000000000000000 8000000000000001 0010000000000000 0000000000000000 0000000000000000 1 1
mixed_1770 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 412e7cb000000000 412e847e00000001 c12e847e00000000 408f400000000000 0000000000000000 7fefffffffffffff c08f400000000000 8000000000000000 bfe0000000000000 3ff0000000000000 0000000000000000 20b0000000000000 1 0
mixed_2170 0000000000000000 0000000000000000 3fe3333333333333 3fe999999999999a 412e848000000000 412e848000000000 412e848000000000 ffefffffffffffff 0170000000000000 8000000000000001 bff0000000000000 c08f400000000000 c08f400000000000 3fe0000000000000 bff0000000000000 0010000000000000 1 0
ordinary_170 3fe67798936c4588 bfe211632887cb34 3fca9de462c0b092 3fd85f3958cc394c 412d85748c9f0d9c c11640f37911a3dc c11655133976d74c c09d52a6dc8a1053 c08ff43600bbfba0 404c0cc759ec7940 c07c94da3d8c67da c0789981e70c40e8 c0266fcb18507000 407983f22cbc22d5 408b68d4d3b5dca2 407459978c430351 1 0
unique_wide_0 0000000000000001 0000000000000002 0000000000000003 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 7fefffffffffffff 7fefffffffffffff 7fefffffffffffff c08f400000000000 c08f400000000000 c08f400000000000 408f400000000000 408f400000000000 408f400000000000 1 0
unique_wide_1 0000000000000002 0000000000000003 0000000000000004 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 7feffffffffffffe 7feffffffffffffe 7feffffffffffffe c08f400000000000 c08f400000000000 c08f400000000000 408f400000000000 408f400000000000 408f400000000000 1 0
unique_cancellation_32767 0000000000008000 0000000000008001 0000000000008002 3ff0000000000000 412e848000000000 412e848000000000 412e848000000000 7fefffffffff8000 7fefffffffff8000 7fefffffffff8000 8000000000000001 8000000000000001 8000000000000001 8000000000000001 8000000000000001 8000000000000001 1 1
broad_17 20a7fef624cf10de 078b5b21f37e3f68 20af244166c9b4bc 3ff0000000000000 412e847e00000000 0000000000000000 412e847e00000000 141fa2654a5c29ac ffb63bcf58398758 142e64f097a20398 8000000000000001 0000000000000001 8000000000000001 0000000000000001 408f400000000000 408f400000000000 1 0
broad_67 be0492ee161774c2 be0c8644b2efbe82 000323c61aeef38a 3ff0000000000000 412e847e00000000 0000000000000000 412e848000000000 000ac74dc5122472 bfec10f440267590 bfe68ab472bc54ea 0000000000000000 c08f400000000000 c08f400000000000 0000000000000000 8000000000000001 0000000000000000 1 0
broad_167 a0995f3c5063bc1c 3e9425ef5f4acb64 3e80a26c97000eb8 bff0000000000000 c12e848000000000 c12e848000000000 412e848000000000 3fd7e0975303da34 800af89b91b498a7 000cf2e84039e3ad 8000000000000001 c08f400000000000 0000000000000000 289c0c4e075343fa 408f400000000000 408f400000000000 1 0
broad_267 399ad34aa049e930 0000000000000001 0000000000000001 3ff0000000000001 c12e848000000000 0000000000000000 c12e848000000000 00094914c602b965 140d2488a799d940 941214bf30c5b9f0 0000000000000001 0000000000000001 c08f400000000000 408f400000000000 0737c90feba9edc0 c08f400000000000 1 1
broad_467 bf41a272c98a5d1c 0000000000000001 0000000000000001 3ff0000000000001 c12e848000000000 412e847e00000001 0000000000000000 0000000000000000 5f28b15861c7c702 ffa11f428faeccf0 408f400000000000 c08f400000000000 0000000000000001 408f400000000000 0fb68b145b8e5068 3f15d2ffbf0127b0 0 0
broad_667 39a77a1fda4fb9d6 3dfbe60d89518c94 3e7738152447e540 bff0000000000000 c12e848000000000 0000000000000000 c12e848000000000 8000000000000001 3fe6a92d5d81ab70 0000000000000001 0000000000000000 c08f400000000000 c08f400000000000 408f400000000000 0214abf47968e336 8000000000000001 1 0
broad_867 0000000000000002 beab4f2f42e7da86 b97b2ff27eac45e0 3ff0000000000001 412e848000000000 0000000000000000 c12e848000000000 bfd50686a014a3f4 3fe4e028176893fe 13f5ccb8bebbf960 c08f400000000000 8000000000000001 0000000000000001 05068bb1ddbf9fe9 408f400000000000 2a5889d98208788d 1 0"#;

    #[test]
    fn independent_eight_corner_rational_vectors() {
        for line in ORACLE_VECTORS.lines() {
            let mut fields = line.split_whitespace();
            let name = fields.next().unwrap();
            let mut next =
                || f64::from_bits(u64::from_str_radix(fields.next().unwrap(), 16).unwrap());
            let c = Case {
                q: std::array::from_fn(|_| next()),
                t: std::array::from_fn(|_| next()),
                s: std::array::from_fn(|_| next()),
                lo: std::array::from_fn(|_| next()),
                hi: std::array::from_fn(|_| next()),
            };
            let valid = fields.next().unwrap() == "1";
            let inside = fields.next().unwrap() == "1";
            let expected = if valid {
                Ok(inside)
            } else {
                Err(TransformSourceError::InvalidQuaternion)
            };
            assert_eq!(exact(c), (valid, inside), "exact {name}");
            assert_eq!(check(c), expected, "filtered {name}");
            if let Some(certified) = filter(c) {
                assert_eq!(certified, (valid, inside), "certificate {name}");
            }
        }
    }

    #[test]
    fn closed_world_extent_and_one_decoded_step_across() {
        for sign in [-1.0, 1.0] {
            let c = Case {
                t: [sign * 999_999.0, 0.0, 0.0],
                ..unit_box()
            };
            assert!(filter(c).is_none());
            assert_eq!(check(c), Ok(true));
            let outside = Case {
                t: [sign * 999_999.0_f64.next_up(), 0.0, 0.0],
                ..c
            };
            assert_eq!(check(outside), Ok(false));
            let inside = Case {
                t: [sign * 999_999.0_f64.next_down(), 0.0, 0.0],
                ..c
            };
            assert_eq!(check(inside), Ok(true));
        }
        assert_eq!(filter(unit_box()), Some((true, true)));
    }

    #[test]
    fn invalid_source_and_quaternion_are_distinct_from_world_refusal() {
        assert_eq!(
            check(Case {
                q: [0.0; 4],
                ..unit_box()
            }),
            Err(TransformSourceError::InvalidQuaternion)
        );
        assert_eq!(
            check(Case {
                q: [f64::MAX; 4],
                ..unit_box()
            }),
            Err(TransformSourceError::InvalidQuaternion)
        );
        assert_eq!(
            check(Case {
                s: [f64::INFINITY; 3],
                ..unit_box()
            }),
            Err(TransformSourceError::InvalidSource)
        );
        assert_eq!(
            check(Case {
                lo: [f64::NAN; 3],
                ..unit_box()
            }),
            Err(TransformSourceError::InvalidSource)
        );
        assert_eq!(
            check(Case {
                lo: [2.0; 3],
                ..unit_box()
            }),
            Err(TransformSourceError::InvalidSource)
        );
        assert_eq!(
            check(Case {
                hi: [1000.0_f64.next_up(); 3],
                ..unit_box()
            }),
            Err(TransformSourceError::InvalidSource)
        );
        assert_eq!(
            check(Case {
                t: [f64::MAX; 3],
                ..unit_box()
            }),
            Ok(false)
        );
        assert_eq!(
            check(Case {
                t: [f64::MAX; 3],
                q: [0.0; 4],
                ..unit_box()
            }),
            Err(TransformSourceError::InvalidQuaternion)
        );
        // Authored translation has its own extent, even if geometry offsets it.
        assert_eq!(
            check(Case {
                t: [1_000_001.0, 0.0, 0.0],
                lo: [-2.0, 0.0, 0.0],
                hi: [-2.0, 0.0, 0.0],
                ..unit_box()
            }),
            Ok(false)
        );
    }

    #[test]
    fn signed_zero_scale_empty_origin_and_quaternion_sign() {
        let c = Case {
            t: [WORLD_EXTENT, -WORLD_EXTENT, WORLD_EXTENT],
            s: [0.0, -0.0, 0.0],
            ..unit_box()
        };
        assert_eq!(check(c), Ok(true));
        let empty = Case {
            s: [f64::MAX; 3],
            lo: [0.0; 3],
            hi: [0.0; 3],
            ..c
        };
        assert_eq!(check(empty), Ok(true));
        let c = Case {
            q: [0.0, 0.0, 0.6, 0.8],
            s: [-2.0, -0.0, -1.0],
            lo: [-3.0, -7.0, 0.0],
            hi: [4.0, 8.0, 0.0],
            ..unit_box()
        };
        assert_eq!(check(c), Ok(true));
        assert_eq!(
            check(Case {
                q: c.q.map(|v| -v),
                ..c
            }),
            check(c)
        );
    }

    #[test]
    fn rounded_equality_and_scale_sign_mutants_disagree() {
        let tiny = f64::from_bits(1);
        let c = Case {
            t: [WORLD_EXTENT, 0.0, 0.0],
            s: [tiny, 1.0, 1.0],
            lo: [0.0; 3],
            hi: [tiny, 0.0, 0.0],
            ..unit_box()
        };
        // Rounded multiplication erases a real positive displacement.
        assert_eq!(c.t[0] + c.s[0] * c.hi[0], WORLD_EXTENT);
        assert!(filter(c).is_none());
        assert_eq!(check(c), Ok(false));
        let negative = Case {
            s: [-1.0, 0.0, 0.0],
            lo: [-1.0, 0.0, 0.0],
            hi: [0.0; 3],
            ..c
        };
        // A mutant that assumes positive coefficients chooses the wrong upper.
        assert!(negative.t[0] + negative.s[0] * negative.hi[0] <= WORLD_EXTENT);
        assert_eq!(check(negative), Ok(false));
    }

    #[test]
    fn source_box_rule_refuses_empty_corner_and_unreferenced_outlier() {
        let c = Case {
            q: [0.0, 0.0, 0.6, 0.8],
            t: [0.0, 999_000.0, 0.0],
            lo: [-1000.0, 0.0, 0.0],
            hi: [1000.0, 1000.0, 0.0],
            ..unit_box()
        };
        assert_eq!(check(c), Ok(false));
        // A vertex-only mutant admits all actual points, missing an empty corner.
        for p in [[1000.0, 0.0, 0.0], [0.0, 1000.0, 0.0], [-1000.0, 0.0, 0.0]] {
            assert_eq!(check(Case { lo: p, hi: p, ..c }), Ok(true));
        }
        let c = Case {
            t: [999_500.0, 0.0, 0.0],
            lo: [0.0; 3],
            hi: [1000.0, 1.0, 0.0],
            ..unit_box()
        };
        assert_eq!(check(c), Ok(false));
        // Dropping an unreferenced extreme from the caller's summary would admit.
        assert_eq!(
            check(Case {
                hi: [1.0, 1.0, 0.0],
                ..c
            }),
            Ok(true)
        );
    }

    #[test]
    fn exact_rotation_divides_by_norm_and_preserves_cancellation() {
        let w = f64::from_bits(0x3ff000000044b82f);
        let c = Case {
            q: [0.0, 0.0, 0.0, w],
            t: [999_999.0, 0.0, 0.0],
            ..unit_box()
        };
        assert_eq!(check(c), Ok(true));
        // Forgetting N/Q applies a spurious scale and breaks closed equality.
        assert!(c.t[0] + w * w > WORLD_EXTENT);
        let tiny = f64::from_bits(1);
        let wide = Case {
            q: [tiny, tiny, tiny, 1.0],
            t: [WORLD_EXTENT; 3],
            s: [f64::MAX; 3],
            lo: [-tiny; 3],
            hi: [-tiny; 3],
        };
        assert!(filter(wide).is_none());
        assert_eq!(check(wide), Ok(true));
        assert_eq!(
            check(Case {
                lo: [tiny; 3],
                hi: [tiny; 3],
                ..wide
            }),
            Ok(false)
        );
    }
}
