//! Dense float vectors plus similarity / distance metrics.
//!
//! The simple methods (`cosine_similarity`, `euclidean_distance`, ...) assume
//! both operands share the same dimension and **zip to the shorter length** if
//! they do not. When you need a hard guarantee, use the `try_*` wrappers which
//! return [`VectorError::DimensionMismatch`] instead.

use serde::{Deserialize, Serialize};

use crate::error::{VectorError, VectorResult};

/// A dense vector of `f32` values (an embedding).
///
/// # Examples
///
/// ```rust
/// use rf_vector::Vector;
///
/// let v = Vector::new(vec![3.0, 4.0]);
/// assert_eq!(v.dimension(), 2);
/// assert_eq!(v.magnitude(), 5.0);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vector(Vec<f32>);

impl Vector {
    /// Create a new vector from a `Vec<f32>`.
    pub fn new(values: Vec<f32>) -> Self {
        Vector(values)
    }

    /// Borrow the underlying values as a slice.
    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }

    /// Number of components in the vector.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` if the vector has no components.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Dimensionality of the vector (alias for [`Vector::len`]).
    pub fn dimension(&self) -> usize {
        self.0.len()
    }

    /// Dot product with another vector.
    ///
    /// Zips to the shorter length if dimensions differ.
    pub fn dot(&self, other: &Vector) -> f32 {
        self.0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| a * b)
            .sum()
    }

    /// L2 (Euclidean) norm — the magnitude / length of the vector.
    pub fn magnitude(&self) -> f32 {
        self.0.iter().map(|x| x * x).sum::<f32>().sqrt()
    }

    /// Cosine similarity in `[-1.0, 1.0]` (higher = more similar).
    ///
    /// Returns `0.0` if either vector has zero magnitude.
    pub fn cosine_similarity(&self, other: &Vector) -> f32 {
        let mag = self.magnitude() * other.magnitude();
        if mag == 0.0 {
            0.0
        } else {
            self.dot(other) / mag
        }
    }

    /// Cosine distance (`1.0 - cosine_similarity`); lower = more similar.
    pub fn cosine_distance(&self, other: &Vector) -> f32 {
        1.0 - self.cosine_similarity(other)
    }

    /// Euclidean (L2) distance between the two vectors.
    pub fn euclidean_distance(&self, other: &Vector) -> f32 {
        self.0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| {
                let d = a - b;
                d * d
            })
            .sum::<f32>()
            .sqrt()
    }

    /// Return a unit-length copy of this vector.
    ///
    /// If the magnitude is `0.0` the vector is returned unchanged.
    pub fn normalized(&self) -> Vector {
        let mag = self.magnitude();
        if mag == 0.0 {
            self.clone()
        } else {
            Vector(self.0.iter().map(|x| x / mag).collect())
        }
    }

    /// Checked dot product: errors if dimensions differ.
    pub fn try_dot(&self, other: &Vector) -> VectorResult<f32> {
        self.check_dims(other)?;
        Ok(self.dot(other))
    }

    /// Checked cosine similarity: errors if dimensions differ.
    pub fn try_cosine_similarity(&self, other: &Vector) -> VectorResult<f32> {
        self.check_dims(other)?;
        Ok(self.cosine_similarity(other))
    }

    /// Checked Euclidean distance: errors if dimensions differ.
    pub fn try_euclidean_distance(&self, other: &Vector) -> VectorResult<f32> {
        self.check_dims(other)?;
        Ok(self.euclidean_distance(other))
    }

    fn check_dims(&self, other: &Vector) -> VectorResult<()> {
        if self.len() != other.len() {
            Err(VectorError::DimensionMismatch {
                left: self.len(),
                right: other.len(),
            })
        } else {
            Ok(())
        }
    }
}

impl From<Vec<f32>> for Vector {
    fn from(values: Vec<f32>) -> Self {
        Vector(values)
    }
}

impl From<&[f32]> for Vector {
    fn from(values: &[f32]) -> Self {
        Vector(values.to_vec())
    }
}

/// How two vectors are compared when ranking search results.
///
/// All metrics expose a [`DistanceMetric::score`] where **a higher score means
/// more similar**, so results can always be sorted in descending order.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum DistanceMetric {
    /// Cosine similarity (angle between vectors). Score = similarity.
    #[default]
    Cosine,
    /// Euclidean (L2) distance. Score = negative distance.
    Euclidean,
    /// Raw dot / inner product. Score = dot product.
    DotProduct,
}

impl DistanceMetric {
    /// Score `a` against `b` such that **higher = more similar**.
    ///
    /// - `Cosine` → cosine similarity (`[-1, 1]`)
    /// - `Euclidean` → `-distance` (so nearer points score higher)
    /// - `DotProduct` → the dot product
    pub fn score(&self, a: &Vector, b: &Vector) -> f32 {
        match self {
            DistanceMetric::Cosine => a.cosine_similarity(b),
            DistanceMetric::Euclidean => -a.euclidean_distance(b),
            DistanceMetric::DotProduct => a.dot(b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_is_one() {
        let a = Vector::new(vec![1.0, 2.0, 3.0]);
        let b = Vector::new(vec![1.0, 2.0, 3.0]);
        assert!((a.cosine_similarity(&b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_is_zero() {
        let a = Vector::new(vec![1.0, 0.0]);
        let b = Vector::new(vec![0.0, 1.0]);
        assert!(a.cosine_similarity(&b).abs() < 1e-6);
    }

    #[test]
    fn cosine_zero_magnitude_returns_zero() {
        let a = Vector::new(vec![0.0, 0.0]);
        let b = Vector::new(vec![1.0, 1.0]);
        assert_eq!(a.cosine_similarity(&b), 0.0);
    }

    #[test]
    fn euclidean_distance_is_correct() {
        let a = Vector::new(vec![0.0, 0.0]);
        let b = Vector::new(vec![3.0, 4.0]);
        assert_eq!(a.euclidean_distance(&b), 5.0);
    }

    #[test]
    fn magnitude_and_normalized() {
        let v = Vector::new(vec![3.0, 4.0]);
        assert_eq!(v.magnitude(), 5.0);
        let n = v.normalized();
        assert!((n.magnitude() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalized_zero_vector_unchanged() {
        let v = Vector::new(vec![0.0, 0.0]);
        assert_eq!(v.normalized(), v);
    }

    #[test]
    fn try_methods_detect_mismatch() {
        let a = Vector::new(vec![1.0, 2.0]);
        let b = Vector::new(vec![1.0, 2.0, 3.0]);
        assert!(matches!(
            a.try_cosine_similarity(&b),
            Err(VectorError::DimensionMismatch { left: 2, right: 3 })
        ));
    }

    #[test]
    fn metric_score_higher_is_closer() {
        let q = Vector::new(vec![1.0, 0.0]);
        let near = Vector::new(vec![1.0, 0.1]);
        let far = Vector::new(vec![-1.0, 0.0]);
        for m in [
            DistanceMetric::Cosine,
            DistanceMetric::Euclidean,
            DistanceMetric::DotProduct,
        ] {
            assert!(m.score(&q, &near) > m.score(&q, &far), "metric {m:?}");
        }
    }

    #[test]
    fn from_conversions() {
        let a: Vector = vec![1.0, 2.0].into();
        let b: Vector = (&[1.0f32, 2.0][..]).into();
        assert_eq!(a, b);
    }
}

#[cfg(test)]
mod adversarial {
    use super::*;

    const EPS: f32 = 1e-6;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    // ---- cosine_similarity ----

    #[test]
    fn cosine_opposite_is_minus_one() {
        let a = Vector::new(vec![1.0, 0.0]);
        let b = Vector::new(vec![-1.0, 0.0]);
        assert!(approx(a.cosine_similarity(&b), -1.0));
        // general opposite (scaled)
        let c = Vector::new(vec![2.0, 3.0, -1.0]);
        let d = Vector::new(vec![-4.0, -6.0, 2.0]); // -2 * c
        assert!(approx(c.cosine_similarity(&d), -1.0));
    }

    #[test]
    fn cosine_both_zero_no_nan() {
        let a = Vector::new(vec![0.0, 0.0]);
        let b = Vector::new(vec![0.0, 0.0]);
        let s = a.cosine_similarity(&b);
        assert_eq!(s, 0.0);
        assert!(s.is_finite());
    }

    #[test]
    fn cosine_either_zero_no_nan() {
        let z = Vector::new(vec![0.0, 0.0, 0.0]);
        let v = Vector::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(z.cosine_similarity(&v), 0.0);
        assert_eq!(v.cosine_similarity(&z), 0.0);
        assert!(z.cosine_similarity(&v).is_finite());
    }

    #[test]
    fn cosine_known_45_degrees() {
        // [1,0] vs [1,1] -> cos(45deg) = 1/sqrt(2)
        let a = Vector::new(vec![1.0, 0.0]);
        let b = Vector::new(vec![1.0, 1.0]);
        assert!(approx(a.cosine_similarity(&b), 1.0 / 2.0f32.sqrt()));
    }

    #[test]
    fn cosine_scale_invariant() {
        let a = Vector::new(vec![1.0, 2.0, 3.0]);
        let b = Vector::new(vec![10.0, 20.0, 30.0]);
        assert!(approx(a.cosine_similarity(&b), 1.0));
    }

    // ---- cosine_distance ----

    #[test]
    fn cosine_distance_is_one_minus_similarity() {
        let pairs = [
            (vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]),
            (vec![1.0, 0.0], vec![0.0, 1.0]),
            (vec![1.0, 0.0], vec![-1.0, 0.0]),
            (vec![0.0, 0.0], vec![1.0, 1.0]),
        ];
        for (l, r) in pairs {
            let a = Vector::new(l);
            let b = Vector::new(r);
            assert!(approx(
                a.cosine_distance(&b),
                1.0 - a.cosine_similarity(&b)
            ));
        }
    }

    // ---- euclidean_distance ----

    #[test]
    fn euclidean_3d_known() {
        // [1,2,3] vs [4,6,3] -> sqrt(9+16+0) = 5
        let a = Vector::new(vec![1.0, 2.0, 3.0]);
        let b = Vector::new(vec![4.0, 6.0, 3.0]);
        assert!(approx(a.euclidean_distance(&b), 5.0));
    }

    #[test]
    fn euclidean_identical_is_zero() {
        let a = Vector::new(vec![1.5, -2.5, 7.0]);
        assert_eq!(a.euclidean_distance(&a), 0.0);
    }

    #[test]
    fn euclidean_symmetric() {
        let a = Vector::new(vec![1.0, 2.0]);
        let b = Vector::new(vec![5.0, 5.0]);
        assert!(approx(
            a.euclidean_distance(&b),
            b.euclidean_distance(&a)
        ));
    }

    // ---- dot / magnitude / normalized ----

    #[test]
    fn dot_known_value() {
        let a = Vector::new(vec![1.0, 2.0, 3.0]);
        let b = Vector::new(vec![4.0, 5.0, 6.0]);
        // 4 + 10 + 18 = 32
        assert!(approx(a.dot(&b), 32.0));
    }

    #[test]
    fn dot_with_self_is_magnitude_squared() {
        let a = Vector::new(vec![3.0, 4.0]);
        assert!(approx(a.dot(&a), a.magnitude() * a.magnitude()));
    }

    #[test]
    fn magnitude_known() {
        assert!(approx(Vector::new(vec![3.0, 4.0]).magnitude(), 5.0));
        assert!(approx(
            Vector::new(vec![1.0, 2.0, 2.0]).magnitude(),
            3.0
        ));
        assert_eq!(Vector::new(vec![0.0, 0.0]).magnitude(), 0.0);
    }

    #[test]
    fn normalized_unit_length_and_direction() {
        let v = Vector::new(vec![3.0, 0.0, 4.0]);
        let n = v.normalized();
        assert!(approx(n.magnitude(), 1.0));
        // direction preserved: cosine with original is 1
        assert!(approx(v.cosine_similarity(&n), 1.0));
        // exact components 3/5, 0, 4/5
        assert!(approx(n.as_slice()[0], 0.6));
        assert!(approx(n.as_slice()[2], 0.8));
    }

    #[test]
    fn normalized_zero_no_nan() {
        let z = Vector::new(vec![0.0, 0.0, 0.0]);
        let n = z.normalized();
        assert_eq!(n, z);
        assert!(n.as_slice().iter().all(|x| x.is_finite()));
    }

    // ---- dimension-mismatch behavior (documented: zip-to-shorter for simple) ----

    #[test]
    fn dot_zips_to_shorter() {
        let a = Vector::new(vec![1.0, 2.0, 3.0, 99.0]);
        let b = Vector::new(vec![4.0, 5.0, 6.0]);
        // ignores 99.0: 4 + 10 + 18 = 32
        assert!(approx(a.dot(&b), 32.0));
    }

    #[test]
    fn euclidean_zips_to_shorter() {
        let a = Vector::new(vec![0.0, 0.0, 100.0]);
        let b = Vector::new(vec![3.0, 4.0]);
        // third component of a ignored -> sqrt(9+16) = 5
        assert!(approx(a.euclidean_distance(&b), 5.0));
    }

    #[test]
    fn try_dot_detects_mismatch() {
        let a = Vector::new(vec![1.0, 2.0]);
        let b = Vector::new(vec![1.0, 2.0, 3.0]);
        assert!(matches!(
            a.try_dot(&b),
            Err(VectorError::DimensionMismatch { left: 2, right: 3 })
        ));
        // matching dims succeed
        let c = Vector::new(vec![1.0, 2.0]);
        assert!(a.try_dot(&c).is_ok());
    }

    #[test]
    fn try_euclidean_detects_mismatch_and_ok() {
        let a = Vector::new(vec![3.0, 0.0]);
        let b = Vector::new(vec![0.0, 4.0]);
        assert!(approx(a.try_euclidean_distance(&b).unwrap(), 5.0));
        let c = Vector::new(vec![1.0]);
        assert!(matches!(
            a.try_euclidean_distance(&c),
            Err(VectorError::DimensionMismatch { left: 2, right: 1 })
        ));
    }

    // ---- DistanceMetric::score ----

    #[test]
    fn euclidean_score_is_negative_distance() {
        let q = Vector::new(vec![0.0, 0.0]);
        let p = Vector::new(vec![3.0, 4.0]);
        let s = DistanceMetric::Euclidean.score(&q, &p);
        assert!(approx(s, -5.0));
        assert!(s < 0.0, "euclidean score must be negative for non-zero distance");
        // identical -> score 0 (the maximum, since distance 0)
        assert_eq!(DistanceMetric::Euclidean.score(&q, &q), 0.0);
    }

    #[test]
    fn cosine_score_equals_similarity() {
        let a = Vector::new(vec![1.0, 2.0]);
        let b = Vector::new(vec![2.0, 1.0]);
        assert!(approx(
            DistanceMetric::Cosine.score(&a, &b),
            a.cosine_similarity(&b)
        ));
    }

    #[test]
    fn dot_score_equals_dot() {
        let a = Vector::new(vec![1.0, 2.0, 3.0]);
        let b = Vector::new(vec![4.0, 5.0, 6.0]);
        assert!(approx(
            DistanceMetric::DotProduct.score(&a, &b),
            a.dot(&b)
        ));
    }

    #[test]
    fn all_metrics_rank_nearer_higher() {
        // A clear near vs far where ALL three metrics must agree on ordering.
        let q = Vector::new(vec![1.0, 0.0]);
        let near = Vector::new(vec![0.95, 0.05]);
        let far = Vector::new(vec![-1.0, 0.0]);
        for m in [
            DistanceMetric::Cosine,
            DistanceMetric::Euclidean,
            DistanceMetric::DotProduct,
        ] {
            assert!(
                m.score(&q, &near) > m.score(&q, &far),
                "metric {m:?}: near={} far={}",
                m.score(&q, &near),
                m.score(&q, &far)
            );
        }
    }

    #[test]
    fn default_metric_is_cosine() {
        assert_eq!(DistanceMetric::default(), DistanceMetric::Cosine);
    }
}
