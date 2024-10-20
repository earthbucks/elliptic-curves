//! 64-bit secp256r1 scalar field algorithms.

use super::MODULUS;
use elliptic_curve::bigint::{Limb, U256};

/// MU = floor(2^512 / n)
///    = 115792089264276142090721624801893421302707618245269942344307673200490803338238
///    = 0x100000000fffffffffffffffeffffffff43190552df1a6c21012ffd85eedf9bfe
const MU: [Limb; 5] = [
    Limb::from_u64(0x012f_fd85_eedf_9bfe),
    Limb::from_u64(0x4319_0552_df1a_6c21),
    Limb::from_u64(0xffff_fffe_ffff_ffff),
    Limb::from_u64(0x0000_0000_ffff_ffff),
    Limb::from_u64(0x0000_0000_0000_0001),
];

/// Barrett Reduction
///
/// The general algorithm is:
/// ```text
/// p = n = order of group
/// b = 2^64 = 64bit machine word
/// k = 4
/// a \in [0, 2^512]
/// mu := floor(b^{2k} / p)
/// q1 := floor(a / b^{k - 1})
/// q2 := q1 * mu
/// q3 := <- floor(a / b^{k - 1})
/// r1 := a mod b^{k + 1}
/// r2 := q3 * m mod b^{k + 1}
/// r := r1 - r2
///
/// if r < 0: r := r + b^{k + 1}
/// while r >= p: do r := r - p (at most twice)
/// ```
///
/// References:
/// - Handbook of Applied Cryptography, Chapter 14
///   Algorithm 14.42
///   http://cacr.uwaterloo.ca/hac/about/chap14.pdf
///
/// - Efficient and Secure Elliptic Curve Cryptography Implementation of Curve P-256
///   Algorithm 6) Barrett Reduction modulo p
///   https://csrc.nist.gov/csrc/media/events/workshop-on-elliptic-curve-cryptography-standards/documents/papers/session6-adalier-mehmet.pdf
#[inline]
#[allow(clippy::too_many_arguments)]
pub(super) const fn barrett_reduce(lo: U256, hi: U256) -> U256 {
    let lo = lo.as_limbs();
    let hi = hi.as_limbs();
    let a0 = lo[0];
    let a1 = lo[1];
    let a2 = lo[2];
    let a3 = lo[3];
    let a4 = hi[0];
    let a5 = hi[1];
    let a6 = hi[2];
    let a7 = hi[3];
    let q1 = [a3, a4, a5, a6, a7];
    let q3 = q1_times_mu_shift_five(&q1);

    let r1 = [a0, a1, a2, a3, a4];
    let r2 = q3_times_n_keep_five(&q3);
    let r = sub_inner_five(r1, r2);

    // Result is in range (0, 3*n - 1),
    // and 90% of the time, no subtraction will be needed.
    let r = subtract_n_if_necessary(r);
    let r = subtract_n_if_necessary(r);
    U256::new([r[0], r[1], r[2], r[3]])
}

const fn q1_times_mu_shift_five(q1: &[Limb; 5]) -> [Limb; 5] {
    // Schoolbook multiplication

    let (_w0, carry) = Limb::ZERO.mac(q1[0], MU[0], Limb::ZERO);
    let (w1, carry) = Limb::ZERO.mac(q1[0], MU[1], carry);
    let (w2, carry) = Limb::ZERO.mac(q1[0], MU[2], carry);
    let (w3, carry) = Limb::ZERO.mac(q1[0], MU[3], carry);
    // NOTE MU[4] == 1
    // let (w4, w5) = Limb::ZERO.mac(q1[0], MU[4], carry);
    let (w4, w5) = Limb::ZERO.adc(q1[0], carry);

    let (_w1, carry) = w1.mac(q1[1], MU[0], Limb::ZERO);
    let (w2, carry) = w2.mac(q1[1], MU[1], carry);
    let (w3, carry) = w3.mac(q1[1], MU[2], carry);
    let (w4, carry) = w4.mac(q1[1], MU[3], carry);
    // let (w5, w6) = mac(w5, q1[1], MU[4], carry);
    let (w5, w6) = w5.adc(q1[1], carry);

    let (_w2, carry) = w2.mac(q1[2], MU[0], Limb::ZERO);
    let (w3, carry) = w3.mac(q1[2], MU[1], carry);
    let (w4, carry) = w4.mac(q1[2], MU[2], carry);
    let (w5, carry) = w5.mac(q1[2], MU[3], carry);
    // let (w6, w7) = w6.mac(q1[2], MU[4], carry);
    let (w6, w7) = w6.adc(q1[2], carry);

    let (_w3, carry) = w3.mac(q1[3], MU[0], Limb::ZERO);
    let (w4, carry) = w4.mac(q1[3], MU[1], carry);
    let (w5, carry) = w5.mac(q1[3], MU[2], carry);
    let (w6, carry) = w6.mac(q1[3], MU[3], carry);
    // let (w7, w8) = w7.mac(q1[3], MU[4], carry);
    let (w7, w8) = w7.adc(q1[3], carry);

    let (_w4, carry) = w4.mac(q1[4], MU[0], Limb::ZERO);
    let (w5, carry) = w5.mac(q1[4], MU[1], carry);
    let (w6, carry) = w6.mac(q1[4], MU[2], carry);
    let (w7, carry) = w7.mac(q1[4], MU[3], carry);
    // let (w8, w9) = w8.mac(q1[4], MU[4], carry);
    let (w8, w9) = w8.adc(q1[4], carry);

    // let q2 = [_w0, _w1, _w2, _w3, _w4, w5, w6, w7, w8, w9];
    [w5, w6, w7, w8, w9]
}

const fn q3_times_n_keep_five(q3: &[Limb; 5]) -> [Limb; 5] {
    // Schoolbook multiplication.

    let modulus = MODULUS.as_limbs();

    let (w0, carry) = Limb::ZERO.mac(q3[0], modulus[0], Limb::ZERO);
    let (w1, carry) = Limb::ZERO.mac(q3[0], modulus[1], carry);
    let (w2, carry) = Limb::ZERO.mac(q3[0], modulus[2], carry);
    let (w3, carry) = Limb::ZERO.mac(q3[0], modulus[3], carry);
    // let (w4, _) = Limb::ZERO.mac(q3[0], 0, carry);
    let (w4, _) = (carry, Limb::ZERO);

    let (w1, carry) = w1.mac(q3[1], modulus[0], Limb::ZERO);
    let (w2, carry) = w2.mac(q3[1], modulus[1], carry);
    let (w3, carry) = w3.mac(q3[1], modulus[2], carry);
    let (w4, _) = w4.mac(q3[1], modulus[3], carry);

    let (w2, carry) = w2.mac(q3[2], modulus[0], Limb::ZERO);
    let (w3, carry) = w3.mac(q3[2], modulus[1], carry);
    let (w4, _) = w4.mac(q3[2], modulus[2], carry);

    let (w3, carry) = w3.mac(q3[3], modulus[0], Limb::ZERO);
    let (w4, _) = w4.mac(q3[3], modulus[1], carry);

    let (w4, _) = w4.mac(q3[4], modulus[0], Limb::ZERO);

    [w0, w1, w2, w3, w4]
}

#[inline]
#[allow(clippy::too_many_arguments)]
const fn sub_inner_five(l: [Limb; 5], r: [Limb; 5]) -> [Limb; 5] {
    let (w0, borrow) = l[0].sbb(r[0], Limb::ZERO);
    let (w1, borrow) = l[1].sbb(r[1], borrow);
    let (w2, borrow) = l[2].sbb(r[2], borrow);
    let (w3, borrow) = l[3].sbb(r[3], borrow);
    let (w4, _borrow) = l[4].sbb(r[4], borrow);

    // If underflow occurred on the final limb - don't care (= add b^{k+1}).
    [w0, w1, w2, w3, w4]
}

#[inline]
#[allow(clippy::too_many_arguments)]
const fn subtract_n_if_necessary(r: [Limb; 5]) -> [Limb; 5] {
    let modulus = MODULUS.as_limbs();

    let (w0, borrow) = r[0].sbb(modulus[0], Limb::ZERO);
    let (w1, borrow) = r[1].sbb(modulus[1], borrow);
    let (w2, borrow) = r[2].sbb(modulus[2], borrow);
    let (w3, borrow) = r[3].sbb(modulus[3], borrow);
    let (w4, borrow) = r[4].sbb(Limb::ZERO, borrow);

    // If underflow occurred on the final limb, borrow = 0xfff...fff, otherwise
    // borrow = 0x000...000. Thus, we use it as a mask to conditionally add the
    // modulus.
    let (w0, carry) = w0.adc(modulus[0].bitand(borrow), Limb::ZERO);
    let (w1, carry) = w1.adc(modulus[1].bitand(borrow), carry);
    let (w2, carry) = w2.adc(modulus[2].bitand(borrow), carry);
    let (w3, carry) = w3.adc(modulus[3].bitand(borrow), carry);
    let (w4, _carry) = w4.adc(Limb::ZERO, carry);

    [w0, w1, w2, w3, w4]
}
