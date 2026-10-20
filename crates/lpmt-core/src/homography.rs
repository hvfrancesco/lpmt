//! Projective 2D transforms used for four-corner surface warping.

use crate::model::Point2;

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum HomographyError {
    #[error("the four point correspondences are degenerate")]
    Degenerate,
}

/// A 3x3 projective transform stored in row-major order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Homography {
    m: [[f64; 3]; 3],
}

impl Homography {
    pub const IDENTITY: Self = Self {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    pub fn from_unit_quad(dst: [Point2; 4]) -> Result<Self, HomographyError> {
        Self::from_quad(
            [
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(1.0, 1.0),
                Point2::new(0.0, 1.0),
            ],
            dst,
        )
    }

    /// Find H such that `H * src_i == dst_i` in homogeneous coordinates.
    pub fn from_quad(src: [Point2; 4], dst: [Point2; 4]) -> Result<Self, HomographyError> {
        let mut equations = [[0.0_f64; 9]; 8];

        for i in 0..4 {
            let x = f64::from(src[i].x);
            let y = f64::from(src[i].y);
            let u = f64::from(dst[i].x);
            let v = f64::from(dst[i].y);

            equations[i * 2] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
            equations[i * 2 + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
        }

        let h = solve_8x8(equations).ok_or(HomographyError::Degenerate)?;
        Ok(Self {
            m: [[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], 1.0]],
        })
    }

    pub fn transform(&self, point: Point2) -> Point2 {
        self.try_transform(point)
            .unwrap_or(Point2::new(f32::NAN, f32::NAN))
    }

    pub fn try_transform(&self, point: Point2) -> Option<Point2> {
        let x = f64::from(point.x);
        let y = f64::from(point.y);
        let w = self.m[2][0] * x + self.m[2][1] * y + self.m[2][2];
        if w.abs() < 1e-12 {
            return None;
        }
        Some(Point2::new(
            ((self.m[0][0] * x + self.m[0][1] * y + self.m[0][2]) / w) as f32,
            ((self.m[1][0] * x + self.m[1][1] * y + self.m[1][2]) / w) as f32,
        ))
    }

    pub fn inverse(&self) -> Result<Self, HomographyError> {
        let [[a, b, c], [d, e, f], [g, h, i]] = self.m;
        let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
        if det.abs() < 1e-12 {
            return Err(HomographyError::Degenerate);
        }

        let inv_det = 1.0 / det;
        Ok(Self {
            m: [
                [
                    (e * i - f * h) * inv_det,
                    (c * h - b * i) * inv_det,
                    (b * f - c * e) * inv_det,
                ],
                [
                    (f * g - d * i) * inv_det,
                    (a * i - c * g) * inv_det,
                    (c * d - a * f) * inv_det,
                ],
                [
                    (d * h - e * g) * inv_det,
                    (b * g - a * h) * inv_det,
                    (a * e - b * d) * inv_det,
                ],
            ],
        })
    }

    /// Column-major `mat4x4` suitable for a WGSL uniform. The z row/column is
    /// unused because all mapping happens on a 2D stage plane.
    pub fn to_column_major_4x4(&self) -> [f32; 16] {
        let m = &self.m;
        [
            m[0][0] as f32,
            m[1][0] as f32,
            0.0,
            m[2][0] as f32,
            m[0][1] as f32,
            m[1][1] as f32,
            0.0,
            m[2][1] as f32,
            0.0,
            0.0,
            1.0,
            0.0,
            m[0][2] as f32,
            m[1][2] as f32,
            0.0,
            m[2][2] as f32,
        ]
    }
}

fn solve_8x8(mut a: [[f64; 9]; 8]) -> Option<[f64; 8]> {
    const EPSILON: f64 = 1e-12;

    for col in 0..8 {
        let pivot = (col..8).max_by(|&left, &right| {
            a[left][col]
                .abs()
                .partial_cmp(&a[right][col].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

        if a[pivot][col].abs() < EPSILON {
            return None;
        }
        a.swap(col, pivot);

        let divisor = a[col][col];
        for cell in &mut a[col][col..=8] {
            *cell /= divisor;
        }

        for row in 0..8 {
            if row == col {
                continue;
            }
            let factor = a[row][col];
            if factor.abs() < EPSILON {
                continue;
            }
            let pivot_row = a[col];
            for (offset, value) in a[row][col..=8].iter_mut().enumerate() {
                *value -= factor * pivot_row[col + offset];
            }
        }
    }

    Some(std::array::from_fn(|i| a[i][8]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: Point2, expected: Point2) {
        assert!(
            (actual.x - expected.x).abs() < 1e-5 && (actual.y - expected.y).abs() < 1e-5,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn identity_is_identity() {
        for point in [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.25, 0.75),
        ] {
            assert_close(Homography::IDENTITY.transform(point), point);
        }
    }

    #[test]
    fn quad_corners_round_trip() {
        let src = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let dst = [
            Point2::new(0.1, 0.2),
            Point2::new(0.9, 0.1),
            Point2::new(0.8, 0.9),
            Point2::new(0.2, 0.7),
        ];

        let h = Homography::from_quad(src, dst).unwrap();
        let inv = h.inverse().unwrap();
        for (source, expected) in src.into_iter().zip(dst) {
            assert_close(h.transform(source), expected);
            assert_close(inv.transform(expected), source);
        }
    }

    #[test]
    fn column_major_4x4_matches_projective_transform() {
        let h = Homography::from_unit_quad([
            Point2::new(0.1, 0.2),
            Point2::new(0.9, 0.1),
            Point2::new(0.8, 0.9),
            Point2::new(0.2, 0.7),
        ])
        .unwrap()
        .inverse()
        .unwrap();
        let m = h.to_column_major_4x4();

        for point in [
            Point2::new(0.1, 0.2),
            Point2::new(0.5, 0.5),
            Point2::new(0.9, 0.8),
        ] {
            let x = m[0] * point.x + m[4] * point.y + m[12];
            let y = m[1] * point.x + m[5] * point.y + m[13];
            let w = m[3] * point.x + m[7] * point.y + m[15];
            assert_close(Point2::new(x / w, y / w), h.transform(point));
        }
    }

    #[test]
    fn degenerate_quad_is_rejected() {
        let dst = [
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 0.0),
        ];
        assert_eq!(
            Homography::from_unit_quad(dst),
            Err(HomographyError::Degenerate)
        );
    }
}
