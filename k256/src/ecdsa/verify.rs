//! ECDSA verifier

use super::{recoverable, Error, Signature};
use crate::{AffinePoint, EncodedPoint, NonZeroScalar, ProjectivePoint, Scalar, Secp256k1};
use ecdsa_core::{hazmat::VerifyPrimitive, signature};
use elliptic_curve::{consts::U32, ops::Invert, FromBytes};
use signature::{digest::Digest, DigestVerifier, PrehashSignature};

/// ECDSA/secp256k1 verification key (i.e. public key)
#[cfg_attr(docsrs, doc(cfg(feature = "ecdsa")))]
pub struct VerifyKey {
    /// Core ECDSA verify key
    key: ecdsa_core::VerifyKey<Secp256k1>,
}

impl VerifyKey {
    /// Initialize [`VerifyKey`] from a SEC1-encoded public key.
    pub fn new(bytes: &[u8]) -> Result<Self, Error> {
        ecdsa_core::VerifyKey::new(bytes).map(|key| VerifyKey { key })
    }

    /// Initialize [`VerifyKey`] from an [`EncodedPoint`].
    pub fn from_encoded_point(public_key: &EncodedPoint) -> Result<Self, Error> {
        ecdsa_core::VerifyKey::from_encoded_point(public_key).map(|key| VerifyKey { key })
    }
}

impl<S> signature::Verifier<S> for VerifyKey
where
    S: PrehashSignature,
    Self: DigestVerifier<S::Digest, S>,
{
    fn verify(&self, msg: &[u8], signature: &S) -> Result<(), Error> {
        self.verify_digest(S::Digest::new().chain(msg), signature)
    }
}

impl<D> DigestVerifier<D, Signature> for VerifyKey
where
    D: Digest<OutputSize = U32>,
{
    fn verify_digest(&self, digest: D, signature: &Signature) -> Result<(), Error> {
        self.key.verify_digest(digest, signature)
    }
}

impl<D> DigestVerifier<D, recoverable::Signature> for VerifyKey
where
    D: Digest<OutputSize = U32>,
{
    fn verify_digest(&self, digest: D, signature: &recoverable::Signature) -> Result<(), Error> {
        self.key.verify_digest(digest, &Signature::from(*signature))
    }
}

impl VerifyPrimitive<Secp256k1> for AffinePoint {
    fn verify_prehashed(&self, z: &Scalar, signature: &Signature) -> Result<(), Error> {
        let maybe_r = NonZeroScalar::from_bytes(signature.r());
        let maybe_s = NonZeroScalar::from_bytes(signature.s());

        // TODO(tarcieri): replace with into conversion when available (see subtle#73)
        let (r, s) = if maybe_r.is_some().into() && maybe_s.is_some().into() {
            (maybe_r.unwrap(), maybe_s.unwrap())
        } else {
            return Err(Error::new());
        };

        // Ensure signature is "low S" normalized ala BIP 0062
        if s.is_high().into() {
            return Err(Error::new());
        }

        let s_inv = s.invert().unwrap();
        let u1 = z * &s_inv;
        let u2 = *r * &s_inv;

        let x = ((&ProjectivePoint::generator() * &u1) + &(ProjectivePoint::from(*self) * &u2))
            .to_affine()
            .unwrap()
            .x;

        if Scalar::from_bytes_reduced(&x.to_bytes()).eq(&r) {
            Ok(())
        } else {
            Err(Error::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{test_vectors::ecdsa::ECDSA_TEST_VECTORS, Secp256k1};
    ecdsa_core::new_verification_test!(Secp256k1, ECDSA_TEST_VECTORS);
}