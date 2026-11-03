//! Private exact decoded-binary64 mesh predicates for the candidate render grammar.
//!
//! These predicates perform no floating-point arithmetic: even their fast path
//! operates on decoded significands. They need no rounding-mode, gradual-underflow
//! or compiler floating-point reassociation assumption. They retain no cache and
//! allocate no heap storage per vertex or triangle. Source JSON decoding, complete
//! F3 admission, charged work and target/record latency remain separate work.
//!
//! Width proof: every accepted local component has |x| <= 1000. With D=2^1074,
//! its integer numerator has magnitude <2^1084; a difference is <2^1085. A cross
//! component has magnitude <2^2171. Radius sums and 10^6*D^2 are <2^2170.
//! 36 u64 limbs provide 2304 bits, including every intermediate and word-rounded
//! multiplication. Removing common powers of two can only reduce these widths.
//! No operation accepts unrestricted finite components into that arithmetic.
//!
//! The exact i128 path aligns canonical significands only when each magnitude
//! is <2^62. Differences are <2^63, products <2^126 and cross differences <2^127.
//! Radius squares sum to <2^126; their comparison limit must fit 127 bits.
//! All signed arithmetic is checked; any failure selects the fixed-width path.

const LIMBS: usize = 36;
const LOCAL_EXTENT: f64 = 1000.0;
const RADIUS_SQUARED: f64 = 1_000_000.0;

/// Nonfinite decoded components cannot be source positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NonFinitePosition;

/// Checks the closed exact decoded-f64 local sphere, including signed zeros.
///
/// The decoder must check every mesh position, including unreferenced vertices.
/// Finite positions outside the sphere return `Ok(false)`, not an F4 refusal.
pub(crate) fn position_within_local_radius(p: [f64; 3]) -> Result<bool, NonFinitePosition> {
    if p.iter().any(|x| !x.is_finite()) {
        return Err(NonFinitePosition);
    }
    if p.iter().any(|x| x.abs() > LOCAL_EXTENT) {
        return Ok(false);
    }
    // The complete cube [-512,512]^3 is strictly inside the sphere. This
    // comparison is exact and avoids any integer construction for that cube.
    if p.iter().all(|x| x.abs() <= 512.0) {
        return Ok(true);
    }
    let exp = common_exponent(p.iter().copied());
    if let Some(inside) = small_radius(p, exp) {
        return Ok(inside);
    }
    let limit = Big::from_f64(RADIUS_SQUARED, 2 * exp);
    let mut sum = Big::default();
    for x in p {
        let v = Big::from_f64(x, exp);
        sum = Big::add(&sum, &Big::mul(&v, &v));
    }
    Ok(sum.abs_cmp(&limit) != std::cmp::Ordering::Greater)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TriangleSourceError {
    IndexOutOfRange,
    RepeatedIndex,
    /// Referenced coordinates must be finite and componentwise within +/-1000.
    InvalidPosition,
}

/// Checks distinct/in-range indices and exact nonzero dyadic cross product.
///
/// Either winding is valid and there is no minimum area. `Ok(false)` means
/// exact geometric degeneracy, even with three distinct indices. The caller
/// must separately run `position_within_local_radius` over ALL mesh positions;
/// this predicate certifies only triangle topology/nondegeneracy. Its coordinate
/// guard enforces arithmetic width even if that caller precondition is missed.
pub(crate) fn triangle_is_nondegenerate(
    positions: &[[f64; 3]],
    indices: [u32; 3],
) -> Result<bool, TriangleSourceError> {
    let mut p = [[0.0; 3]; 3];
    for (dst, index) in p.iter_mut().zip(indices) {
        let i = usize::try_from(index).map_err(|_| TriangleSourceError::IndexOutOfRange)?;
        *dst = *positions
            .get(i)
            .ok_or(TriangleSourceError::IndexOutOfRange)?;
    }
    if indices[0] == indices[1] || indices[0] == indices[2] || indices[1] == indices[2] {
        return Err(TriangleSourceError::RepeatedIndex);
    }
    if p.iter()
        .flatten()
        .any(|x| !x.is_finite() || x.abs() > LOCAL_EXTENT)
    {
        return Err(TriangleSourceError::InvalidPosition);
    }
    let signs = small_cross(p).unwrap_or_else(|| exact_cross(p));
    Ok(signs.iter().any(|&s| s != 0))
}

/// Canonical integer significand, its power of two, and sign. Zero has no sign.
fn parts(x: f64) -> (u64, i32, bool) {
    let bits = x.to_bits();
    let fraction = bits & ((1u64 << 52) - 1);
    let raw = ((bits >> 52) & 0x7ff) as i32;
    let (mantissa, exp) = if raw == 0 {
        (fraction, -1074)
    } else {
        ((1u64 << 52) | fraction, raw - 1023 - 52)
    };
    if mantissa == 0 {
        return (0, 0, false);
    }
    let zeros = mantissa.trailing_zeros();
    (mantissa >> zeros, exp + zeros as i32, bits >> 63 != 0)
}

fn common_exponent(values: impl Iterator<Item = f64>) -> i32 {
    values
        .filter_map(|x| {
            let (mantissa, exp, _) = parts(x);
            (mantissa != 0).then_some(exp)
        })
        .min()
        .unwrap_or(0)
        .min(0)
}

fn small_integer(x: f64, exp: i32, max_bits: u32) -> Option<i128> {
    let (mantissa, e, neg) = parts(x);
    if mantissa == 0 {
        return Some(0);
    }
    let shift = u32::try_from(e - exp).ok()?;
    // Check total bit width BEFORE shifting; checked_shl alone does not
    // detect shifted-out value bits. All callers request at most 127 bits.
    if (64 - mantissa.leading_zeros()).checked_add(shift)? > max_bits {
        return None;
    }
    let v = i128::from(mantissa).checked_shl(shift)?;
    if neg { v.checked_neg() } else { Some(v) }
}

fn small_radius(p: [f64; 3], exp: i32) -> Option<bool> {
    let limit = small_integer(RADIUS_SQUARED, 2 * exp, 127)?;
    let mut sum = 0i128;
    for x in p {
        let v = small_integer(x, exp, 62)?;
        sum = sum.checked_add(v.checked_mul(v)?)?;
    }
    Some(sum <= limit)
}

fn small_cross(p: [[f64; 3]; 3]) -> Option<[i8; 3]> {
    let exp = common_exponent(p.iter().flatten().copied());
    let mut values = [[0i128; 3]; 3];
    for (dst, x) in values.iter_mut().flatten().zip(p.into_iter().flatten()) {
        *dst = small_integer(x, exp, 62)?;
    }
    let mut d = [[0i128; 3]; 2];
    for i in 0..2 {
        for j in 0..3 {
            d[i][j] = values[i + 1][j].checked_sub(values[0][j])?;
        }
    }
    let mut signs = [0; 3];
    for (k, sign) in signs.iter_mut().enumerate() {
        let a = (k + 1) % 3;
        let b = (k + 2) % 3;
        let v = d[0][a]
            .checked_mul(d[1][b])?
            .checked_sub(d[0][b].checked_mul(d[1][a])?)?;
        *sign = v.signum() as i8;
    }
    Some(signs)
}

fn exact_cross(p: [[f64; 3]; 3]) -> [i8; 3] {
    let exp = common_exponent(p.iter().flatten().copied());
    let mut d = [[Big::default(); 3]; 2];
    for i in 0..2 {
        for j in 0..3 {
            d[i][j] = Big::sub(
                &Big::from_f64(p[i + 1][j], exp),
                &Big::from_f64(p[0][j], exp),
            );
        }
    }
    std::array::from_fn(|k| {
        let a = (k + 1) % 3;
        let b = (k + 2) % 3;
        Big::sub(&Big::mul(&d[0][a], &d[1][b]), &Big::mul(&d[0][b], &d[1][a])).sign
    })
}

#[derive(Clone, Copy)]
struct Big {
    sign: i8,
    n: usize,
    limb: [u64; LIMBS],
}
impl Default for Big {
    fn default() -> Self {
        Self {
            sign: 0,
            n: 0,
            limb: [0; LIMBS],
        }
    }
}

impl Big {
    fn from_f64(x: f64, common_exp: i32) -> Self {
        assert!(x.is_finite());
        let (mant, exp, neg) = parts(x);
        if mant == 0 {
            return Self::default();
        }
        let shift = usize::try_from(exp - common_exp).expect("nonnegative dyadic alignment");
        let word = shift / 64;
        let offset = shift % 64;
        assert!(word + 1 < LIMBS);
        let mut result = Self::default();
        result.limb[word] = mant << offset;
        if offset != 0 {
            result.limb[word + 1] = mant >> (64 - offset);
        }
        result.n = if result.limb[word + 1] == 0 {
            word + 1
        } else {
            word + 2
        };
        result.sign = if neg { -1 } else { 1 };
        result
    }

    fn trim(&mut self) {
        while self.n > 0 && self.limb[self.n - 1] == 0 {
            self.n -= 1;
        }
        if self.n == 0 {
            self.sign = 0;
        }
    }

    fn abs_cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        match self.n.cmp(&rhs.n) {
            std::cmp::Ordering::Equal => {
                for i in (0..self.n).rev() {
                    let c = self.limb[i].cmp(&rhs.limb[i]);
                    if c != std::cmp::Ordering::Equal {
                        return c;
                    }
                }
                std::cmp::Ordering::Equal
            }
            c => c,
        }
    }

    fn abs_add(a: &Self, b: &Self) -> Self {
        let mut out = Self::default();
        let mut carry: u128 = 0;
        let n = a.n.max(b.n);
        assert!(n < LIMBS);
        for i in 0..n {
            let sum = a.limb[i] as u128 + b.limb[i] as u128 + carry;
            out.limb[i] = sum as u64;
            carry = sum >> 64;
        }
        out.n = n;
        if carry != 0 {
            out.limb[n] = carry as u64;
            out.n += 1;
        }
        out.trim();
        out
    }

    fn abs_sub(a: &Self, b: &Self) -> Self {
        assert!(a.abs_cmp(b) != std::cmp::Ordering::Less);
        let mut out = Self::default();
        let mut borrow: u128 = 0;
        for i in 0..a.n {
            let ai = a.limb[i] as u128;
            let bi = b.limb[i] as u128 + borrow;
            out.limb[i] = ai.wrapping_sub(bi) as u64;
            borrow = u128::from(ai < bi);
        }
        assert_eq!(borrow, 0);
        out.n = a.n;
        out.trim();
        out
    }

    fn add(a: &Self, b: &Self) -> Self {
        if a.sign == 0 {
            return *b;
        }
        if b.sign == 0 {
            return *a;
        }
        if a.sign == b.sign {
            let mut out = Self::abs_add(a, b);
            out.sign = a.sign;
            out
        } else {
            match a.abs_cmp(b) {
                std::cmp::Ordering::Greater => {
                    let mut out = Self::abs_sub(a, b);
                    if out.n > 0 {
                        out.sign = a.sign;
                    }
                    out
                }
                std::cmp::Ordering::Less => {
                    let mut out = Self::abs_sub(b, a);
                    if out.n > 0 {
                        out.sign = b.sign;
                    }
                    out
                }
                std::cmp::Ordering::Equal => Self::default(),
            }
        }
    }

    fn sub(a: &Self, b: &Self) -> Self {
        let mut neg = *b;
        neg.sign = -neg.sign;
        Self::add(a, &neg)
    }

    fn mul(a: &Self, b: &Self) -> Self {
        if a.sign == 0 || b.sign == 0 {
            return Self::default();
        }
        assert!(a.n + b.n <= LIMBS);
        let mut out = Self {
            sign: a.sign * b.sign,
            ..Self::default()
        };
        for i in 0..a.n {
            let mut carry: u128 = 0;
            for j in 0..b.n {
                let k = i + j;
                let sum = (a.limb[i] as u128) * (b.limb[j] as u128) + out.limb[k] as u128 + carry;
                out.limb[k] = sum as u64;
                carry = sum >> 64;
            }
            let mut k = i + b.n;
            while carry != 0 {
                assert!(k < LIMBS);
                let sum = out.limb[k] as u128 + carry;
                out.limb[k] = sum as u64;
                carry = sum >> 64;
                k += 1;
            }
        }
        out.n = a.n + b.n;
        out.trim();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independently computed Python Fraction signs retained from the archived
    // R0-F3-EXACT-TRIANGLE-PROBE-2026-11-03 cases.tsv: first nine and stride 251
    // from case 20. The standalone archive harness additionally checks all 8009.
    const TRIANGLE_ORACLE: &str = r#"0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 0000000000000000 0 0 1
0000000000000000 0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 0000000000000000 3ff0000000000000 0000000000000000 0000000000000000 0 0 -1
0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 0000000000000000 3ff0000000000000 3ff0000000000001 0000000000000001 1 -1 1
0000000000000000 0000000000000000 0000000000000000 3ff0000000000001 3ff0000000000000 0000000000000000 3ff0000000000000 3fefffffffffffff 0000000000000000 0 0 1
0000000000000000 0000000000000000 0000000000000000 0000000000000001 0000000000000000 0000000000000000 0000000000000000 0000000000000001 0000000000000000 0 0 1
0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 0000000000000001 1 -1 0
0000000000000000 0000000000000000 0000000000000000 3ff0000000000000 3ff0000000000000 0000000000000000 4000000000000000 4000000000000000 0000000000000000 0 0 0
8000000000000000 0000000000000000 0000000000000000 8000000000000000 0000000000000000 0000000000000000 0000000000000001 0000000000000000 0000000000000000 0 0 0
c08f400000000000 c08f400000000000 c08f400000000000 408f400000000000 408f400000000000 408f400000000000 408f400000000000 3ff0000000000001 0000000000000001 -1 1 -1
8000000000000000 0000000000000001 8000000000000001 3ff0000000000000 4080000000000000 0000000000000001 3ff0000000000001 8000000000000000 bff0000000000000 -1 1 -1
3fefffffffffffff 20b0000000000000 0000000000000001 8000000000000001 c08f400000000000 c080000000000000 c08f400000000000 8000000000000001 3ff0000000000001 -1 1 -1
3fefffffffffffff c080000000000000 0000000000000001 0010000000000000 4080000000000000 8000000000000000 408f400000000000 3ff0000000000001 0000000000000000 -1 -1 -1
8000000000000000 3ff0000000000000 0010000000000000 0010000000000000 3fefffffffffffff 8000000000000000 408f400000000000 0010000000000000 c08f400000000000 1 1 1
20b0000000000000 20b0000000000000 20b0000000000000 4080000000000000 3ff0000000000000 0000000000000001 20b0000000000000 0000000000000001 8000000000000000 -1 1 -1
c08f400000000000 3ff0000000000001 3ff0000000000000 4080000000000000 0000000000000001 3ff0000000000001 c080000000000000 8000000000000000 3fefffffffffffff 1 1 -1
0000000000000000 408f400000000000 bff0000000000000 20b0000000000000 0010000000000000 8000000000000000 0000000000000001 408f400000000000 bff0000000000000 0 1 1
4080000000000000 0000000000000001 4080000000000000 bff0000000000000 20b0000000000000 0000000000000000 0000000000000001 c080000000000000 0010000000000000 -1 -1 1
0000000000000001 c08f400000000000 3ff0000000000001 0000000000000000 0000000000000000 8000000000000001 0010000000000000 3ff0000000000001 bff0000000000000 -1 -1 -1
408f400000000000 8000000000000000 408f400000000000 20b0000000000000 3fefffffffffffff 3ff0000000000000 0000000000000001 0000000000000001 408f400000000000 1 1 1
bff0000000000000 4080000000000000 8000000000000000 8000000000000001 4080000000000000 3ff0000000000000 0000000000000001 0000000000000000 408f400000000000 1 -1 -1
0000000000000001 c08f400000000000 3fefffffffffffff 8000000000000000 408f400000000000 0000000000000001 c08f400000000000 0000000000000000 8000000000000001 -1 1 1
0000000000000001 8000000000000000 408f400000000000 c08f400000000000 8000000000000001 4080000000000000 c080000000000000 3fefffffffffffff 8000000000000001 1 -1 -1
20b0000000000000 0000000000000001 0010000000000000 0010000000000000 0010000000000000 20b0000000000000 4080000000000000 8000000000000000 20b0000000000000 1 1 -1
3ff0000000000001 20b0000000000000 8000000000000001 c080000000000000 c080000000000000 0000000000000001 3ff0000000000000 c08f400000000000 3ff0000000000000 -1 1 1
bff0000000000000 20b0000000000000 8000000000000001 8000000000000001 0000000000000000 20b0000000000000 0010000000000000 bff0000000000000 0010000000000000 1 1 -1
0000000000000000 0000000000000000 408f400000000000 20b0000000000000 408f400000000000 4080000000000000 20b0000000000001 408f3fffffffffff 4080000000000001 1 -1 -1
4080000000000000 c080000000000000 408f400000000000 3ff0000000000000 8000000000000001 0010000000000000 3fefffffffffffff 8000000000000002 000fffffffffffff -1 1 1
8000000000000000 c08f400000000000 0000000000000000 bff0000000000000 3ff0000000000001 408f400000000000 bff0000000000001 3ff0000000000000 408f400000000000 1 -1 1
8000000000000001 0010000000000000 4080000000000000 3ff0000000000000 c08f400000000000 0010000000000000 3ff0000000000001 c08f3fffffffffff 0010000000000001 1 -1 1
8000000000000000 8000000000000000 0000000000000001 408f400000000000 0000000000000000 0010000000000000 408f400000000000 8000000000000001 0010000000000001 1 -1 -1
bff0000000000000 3ff0000000000001 8000000000000000 408f400000000000 0000000000000001 8000000000000000 408f3fffffffffff 0000000000000000 0000000000000001 -1 -1 -1
0000000000000001 bff0000000000000 4080000000000000 bff0000000000000 408f400000000000 3ff0000000000000 bfefffffffffffff 408f3fffffffffff 3ff0000000000001 -1 -1 1
3ff0000000000001 0000000000000000 8000000000000001 c080000000000000 3fefffffffffffff 20b0000000000000 c080000000000001 3feffffffffffffe 20b0000000000001 1 1 1
c080000000000000 3ff0000000000000 8000000000000000 c08f400000000000 3ff0000000000001 0010000000000000 c08f3fffffffffff 3ff0000000000000 000fffffffffffff 1 1 1
c080000000000000 c08f400000000000 3ff0000000000001 20b0000000000000 0000000000000000 3fefffffffffffff 20afffffffffffff 0000000000000001 3ff0000000000000 1 -1 1
0000000000000001 c08f400000000000 0000000000000001 bff0000000000000 c08f400000000000 408f400000000000 bfefffffffffffff c08f400000000000 408f400000000000 0 1 0
3fefffffffffffff 0010000000000000 0000000000000001 408f400000000000 8000000000000001 3ff0000000000001 408f400000000000 8000000000000000 3ff0000000000002 -1 -1 1
c080000000000000 bff0000000000000 3ff0000000000001 bff0000000000000 3fefffffffffffff bff0000000000000 bfefffffffffffff 3feffffffffffffe bff0000000000001 -1 1 -1
3ff0000000000001 20b0000000000000 0010000000000000 0000000000000001 0010000000000000 3fefffffffffffff 0000000000000002 0010000000000001 3ff0000000000000 -1 1 -1
0010000000000000 3ff0000000000000 0000000000000000 0000000000000000 0000000000000001 bff0000000000000 8000000000000001 0000000000000002 bfefffffffffffff -1 1 -1
c08f400000000000 0000000000000000 3fefffffffffffff 3ff0000000000001 3ff0000000000001 4080000000000000 3ff0000000000002 3ff0000000000000 4080000000000001 1 -1 -1"#;

    // Independent fractions.Fraction.from_float squared-sum oracle. Includes
    // equality and adjacent decoded values around 600/800 and 1000 boundaries.
    const RADIUS_ORACLE: &str = r#"0000000000000000 8000000000000000 0000000000000000 1
408f400000000000 0000000000000000 0000000000000000 1
408f400000000000 0000000000000001 0000000000000000 0
4082c00000000000 4089000000000000 0000000000000000 1
4082c00000000000 4089000000000000 3eb0c6f7a0b5ed8d 0
408f400000000000 3ff0000000000000 0000000000000000 0
c08f400000000000 0000000000000000 0000000000000000 1
4080000000000000 4080000000000000 4080000000000000 1
408f400000000000 408f400000000000 408f400000000000 0
01a56e1fc2f8f359 01a56e1fc2f8f359 01a56e1fc2f8f359 1
4082bfffffffffff 4088ffffffffffff 0000000000000000 1
4082bfffffffffff 4089000000000000 0000000000000000 1
4082bfffffffffff 4089000000000001 0000000000000000 0
4082c00000000000 4088ffffffffffff 0000000000000000 1
4082c00000000000 4089000000000000 0000000000000000 1
4082c00000000000 4089000000000001 0000000000000000 0
4082c00000000001 4088ffffffffffff 0000000000000000 1
4082c00000000001 4089000000000000 0000000000000000 0
4082c00000000001 4089000000000001 0000000000000000 0
4088ffffffffffff 4082bfffffffffff 0000000000000000 1
4088ffffffffffff 4082c00000000000 0000000000000000 1
4088ffffffffffff 4082c00000000001 0000000000000000 1
4089000000000000 4082bfffffffffff 0000000000000000 1
4089000000000000 4082c00000000000 0000000000000000 1
4089000000000000 4082c00000000001 0000000000000000 0
4089000000000001 4082bfffffffffff 0000000000000000 0
4089000000000001 4082c00000000000 0000000000000000 0
4089000000000001 4082c00000000001 0000000000000000 0
408f3fffffffffff 8000000000000001 0000000000000000 1
408f3fffffffffff 0000000000000000 0000000000000000 1
408f3fffffffffff 0000000000000001 0000000000000000 1
408f400000000000 8000000000000001 0000000000000000 0
408f400000000000 0000000000000000 0000000000000000 1
408f400000000000 0000000000000001 0000000000000000 0
408f400000000001 8000000000000001 0000000000000000 0
408f400000000001 0000000000000000 0000000000000000 0
408f400000000001 0000000000000001 0000000000000000 0"#;

    fn number(fields: &mut std::str::SplitWhitespace<'_>) -> f64 {
        f64::from_bits(u64::from_str_radix(fields.next().unwrap(), 16).unwrap())
    }

    #[test]
    fn archived_independent_triangle_signs_and_both_windings() {
        for (i, line) in TRIANGLE_ORACLE.lines().enumerate() {
            let mut fields = line.split_whitespace();
            let p = std::array::from_fn(|_| std::array::from_fn(|_| number(&mut fields)));
            let expected: [i8; 3] =
                std::array::from_fn(|_| fields.next().unwrap().parse().unwrap());
            assert_eq!(exact_cross(p), expected, "fallback case {i}");
            if let Some(fast) = small_cross(p) {
                assert_eq!(fast, expected, "fast case {i}");
            }
            let nonzero = expected.iter().any(|&s| s != 0);
            assert_eq!(
                triangle_is_nondegenerate(&p, [0, 1, 2]),
                Ok(nonzero),
                "entry case {i}"
            );
            assert_eq!(
                triangle_is_nondegenerate(&p, [0, 2, 1]),
                Ok(nonzero),
                "reverse case {i}"
            );
        }
    }

    #[test]
    fn independent_radius_boundaries_and_component_permutations() {
        for (i, line) in RADIUS_ORACLE.lines().enumerate() {
            let mut fields = line.split_whitespace();
            let p: [f64; 3] = std::array::from_fn(|_| number(&mut fields));
            let expected = fields.next().unwrap() == "1";
            for p in [p, [p[1], p[2], p[0]], p.map(|x| -x)] {
                assert_eq!(position_within_local_radius(p), Ok(expected), "case {i}");
            }
        }
    }

    #[test]
    fn malformed_indices_and_degenerate_geometry_are_distinct() {
        let p = [[0.0; 3], [1.0; 3], [2.0; 3], [1.0; 3]];
        assert_eq!(
            triangle_is_nondegenerate(&p, [0, 1, 4]),
            Err(TriangleSourceError::IndexOutOfRange)
        );
        assert_eq!(
            triangle_is_nondegenerate(&p, [0, 1, u32::MAX]),
            Err(TriangleSourceError::IndexOutOfRange)
        );
        assert_eq!(
            triangle_is_nondegenerate(&[], [0, 1, 2]),
            Err(TriangleSourceError::IndexOutOfRange)
        );
        assert_eq!(
            triangle_is_nondegenerate(&p, [0, 0, 1]),
            Err(TriangleSourceError::RepeatedIndex)
        );
        assert_eq!(triangle_is_nondegenerate(&p, [0, 1, 2]), Ok(false));
        assert_eq!(triangle_is_nondegenerate(&p, [0, 1, 3]), Ok(false));
    }

    #[test]
    fn invalid_coordinates_do_not_enter_bounded_arithmetic() {
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                position_within_local_radius([x, 0.0, 0.0]),
                Err(NonFinitePosition)
            );
        }
        for x in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            -f64::MAX,
            1000.0_f64.next_up(),
        ] {
            let p = [[0.0; 3], [x, 0.0, 0.0], [0.0, 1.0, 0.0]];
            assert_eq!(
                triangle_is_nondegenerate(&p, [0, 1, 2]),
                Err(TriangleSourceError::InvalidPosition)
            );
            if x.is_finite() {
                assert_eq!(position_within_local_radius(p[1]), Ok(false));
            }
        }
    }

    #[test]
    fn no_area_floor_rounded_zero_or_component_cube_mutant() {
        let tiny = f64::from_bits(1);
        let p = [[0.0; 3], [tiny, 0.0, 0.0], [0.0, tiny, 0.0]];
        assert_eq!(tiny * tiny, 0.0); // Rounded f64 cross would reject this.
        assert_eq!(triangle_is_nondegenerate(&p, [0, 1, 2]), Ok(true));
        let p = [[0.0; 3], [1.0, 1.0, 0.0], [1.0, 1.0_f64.next_up(), tiny]];
        assert!(small_cross(p).is_none());
        assert_eq!(exact_cross(p), [1, -1, 1]);
        assert_eq!(triangle_is_nondegenerate(&p, [0, 1, 2]), Ok(true));
        let p = [600.0, 800.0, 1e-6];
        assert_eq!(p.iter().map(|x| x * x).sum::<f64>(), RADIUS_SQUARED);
        assert_eq!(position_within_local_radius(p), Ok(false));
        assert_eq!(position_within_local_radius([1000.0, tiny, 0.0]), Ok(false));
        assert_eq!(position_within_local_radius([1000.0, 1.0, 0.0]), Ok(false));
    }

    #[test]
    fn exact_integer_fast_path_width_boundary() {
        let unit = f64::from_bits((1023_u64 - 52) << 52);
        let p = [[-512.0, 0.0, 0.0], [512.0, unit, 0.0], [0.0, 0.0, unit]];
        assert_eq!(small_cross(p), Some(exact_cross(p)));
        let half = f64::from_bits((1023_u64 - 53) << 52);
        let wide = [[-512.0, 0.0, 0.0], [512.0, half, 0.0], [0.0, 0.0, half]];
        assert!(small_cross(wide).is_none());
        assert_eq!(triangle_is_nondegenerate(&wide, [0, 1, 2]), Ok(true));
    }

    #[test]
    fn archived_authored_fixture_geometry_and_unreferenced_vertex_rule() {
        // Exact decoded positions from R0-AUTHORED-FIXTURE-2026-11-03.
        let meshes: [[[f64; 3]; 4]; 3] = [
            [
                [-3.0, -2.0, 0.0],
                [3.0, -2.0, 0.0],
                [3.0, 2.0, 0.0],
                [-3.0, 2.0, 0.0],
            ],
            [
                [-0.6, -1.0, 1.0],
                [0.6, -1.0, 1.0],
                [0.6, 1.0, 1.0],
                [-0.6, 1.0, 1.0],
            ],
            [
                [3.4, -0.4, 2.0],
                [3.8, -0.4, 2.0],
                [3.8, 0.4, 2.0],
                [3.4, 0.4, 2.0],
            ],
        ];
        for p in meshes {
            for v in p {
                assert_eq!(position_within_local_radius(v), Ok(true));
            }
            for indices in [[0, 1, 2], [0, 2, 3], [0, 2, 1], [0, 3, 2]] {
                assert_eq!(triangle_is_nondegenerate(&p, indices), Ok(true));
            }
        }
        let p = [
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1000.0, 1.0, 0.0],
        ];
        assert_eq!(triangle_is_nondegenerate(&p, [0, 1, 2]), Ok(true));
        assert_eq!(position_within_local_radius(p[3]), Ok(false));
    }
}
