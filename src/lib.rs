// SPDX-License-Identifier: MPL-2.0

//! Image deformation using moving least squares.

#![warn(missing_docs)]

use core::ops::{Add, Mul, Sub};

/// Move a given point from its original position to its new position
/// according to the deformation that transforms the original control points
/// into their displaced locations.
///
/// The estimated transformation is an affine 2d transformation.
fn deform_affine(
    controls_p: &[(f32, f32)], // p in the paper
    controls_q: &[(f32, f32)], // q in the paper
    point: (f32, f32),         // v in the paper
    proximity_threshold: f32,  // if v is too close to a control point p, we return its associated q
) -> (f32, f32) {
    let v = Point::from(point);
    let sqr_dist = |p: Point| (p - v).sqr_norm();

    // The weight of a given control point depends on its distance to the current point.
    // CAREFUL: this w can go to infinity.
    let weight = |pt| 1.0 / sqr_dist(pt);
    let w_all: Vec<_> = controls_p.iter().map(|&p| weight(p.into())).collect();
    let w_sum: f32 = w_all.iter().sum();
    if w_sum.is_infinite() {
        // Most probably, at least one of the weights is infinite,
        // because our point basically coincide with a control point.
        let index = w_all
            .iter()
            .position(|w| w.is_infinite())
            .expect("There is an infinite sum of the weights but none is infinite");
        return controls_q[index];
    }

    // Compute the centroid p*.
    let wp_star_sum: Point = w_all
        .iter()
        .zip(controls_p)
        .map(|(&w, &p)| w * Point::from(p))
        .fold(Point::zero(), |wp_sum, wp| wp_sum + wp);
    let p_star = (1.0 / w_sum) * wp_star_sum;

    // Compute the centroid q*.
    let wq_star_sum: Point = w_all
        .iter()
        .zip(controls_q)
        .map(|(&w, &q)| w * Point::from(q))
        .fold(Point::zero(), |wq_sum, wq| wq_sum + wq);
    let q_star = (1.0 / w_sum) * wq_star_sum;

    // Compute the affine matrix M.
    let p_hat: Vec<Point> = controls_p
        .iter()
        .map(|&p| Point::from(p) - p_star)
        .collect();
    // m_p_hat is a 2x2 matrix.
    let mp: Mat2 = w_all
        .iter()
        .zip(&p_hat)
        .map(|(&w, &p)| w * p.times_transpose(p))
        .fold(Mat2::zero(), |mp_sum, wpp| mp_sum + wpp);
    // Compute the second part of M.
    let mq: Mat2 = w_all
        .iter()
        .zip(&p_hat)
        .zip(controls_q)
        .map(|((&w, &ph), &q)| {
            let qh = Point::from(q) - q_star;
            (w * ph).times_transpose(qh)
        })
        .fold(Mat2::zero(), |mq_sum, pq| mq_sum + pq);
    // Compute actual coefficients of M.
    let m = mp.inv() * mq;

    // Finally compute the projection of our original point.
    let vx_star: f32;
    todo!()
}

// 2D points helper ############################################################
// That's to avoid a dependency on a heavy package such as nalgebra

/// Point represented by a 2x1 column vector.
#[derive(Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
}

impl Point {
    /// 0
    fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Dot product with another point.
    fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    /// Square norm.
    fn sqr_norm(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Create a 2x2 matrix from a 2x1 point
    fn times_transpose(self, rhs: Self) -> Mat2 {
        Mat2 {
            m11: self.x * rhs.x,
            m21: self.y * rhs.x,
            m12: self.x * rhs.y,
            m22: self.y * rhs.y,
        }
    }
}

// Convert from (x,y) to Point { x, y }
impl From<(f32, f32)> for Point {
    fn from((x, y): (f32, f32)) -> Self {
        Point { x, y }
    }
}

// Add two points
impl Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

// Substract a point
impl Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

// Scalar multiplication
impl Mul<Point> for f32 {
    type Output = Point;
    fn mul(self, rhs: Point) -> Self::Output {
        Point {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}

// 2x2 matrix helper ###########################################################
// That's to avoid a dependency on a heavy package such as nalgebra

/// 2x2 Matrix with the following coefficients.
///
/// | m11  m12 |
/// | m21  m22 |
struct Mat2 {
    m11: f32,
    m21: f32,
    m12: f32,
    m22: f32,
}

impl Mat2 {
    /// 0
    fn zero() -> Self {
        Self {
            m11: 0.0,
            m21: 0.0,
            m12: 0.0,
            m22: 0.0,
        }
    }

    /// Determinant
    fn det(self) -> f32 {
        self.m11 * self.m22 - self.m21 * self.m12
    }

    /// Inverse of a matrix (does not check if det is 0)
    fn inv(self) -> Self {
        1.0 / self.det()
            * Self {
                m11: self.m22,
                m21: -self.m21,
                m12: -self.m12,
                m22: self.m11,
            }
    }
}

// Add two matrices
impl Add for Mat2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            m11: self.m11 + rhs.m11,
            m21: self.m21 + rhs.m21,
            m12: self.m12 + rhs.m12,
            m22: self.m22 + rhs.m22,
        }
    }
}

// Scalar multiplication
impl Mul<Mat2> for f32 {
    type Output = Mat2;
    fn mul(self, rhs: Mat2) -> Self::Output {
        Mat2 {
            m11: self * rhs.m11,
            m21: self * rhs.m21,
            m12: self * rhs.m12,
            m22: self * rhs.m22,
        }
    }
}

// Matrix multiplication
impl Mul for Mat2 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Mat2 {
            m11: self.m11 * rhs.m11 + self.m12 * rhs.m21,
            m21: self.m21 * rhs.m11 + self.m22 * rhs.m21,
            m12: self.m11 * rhs.m12 + self.m12 * rhs.m22,
            m22: self.m21 * rhs.m12 + self.m22 * rhs.m22,
        }
    }
}
