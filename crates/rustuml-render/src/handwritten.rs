// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Write;

use crate::plantuml_metrics as pm;

#[derive(Clone, Copy)]
pub(crate) struct JavaRandom {
    seed: i64,
}

impl JavaRandom {
    const MULTIPLIER: i64 = 0x5DEECE66D;
    const ADDEND: i64 = 0xB;
    const MASK: i64 = (1 << 48) - 1;

    pub(crate) fn new(seed: i64) -> Self {
        Self {
            seed: (seed ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::ADDEND)
            & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }

    fn next_double(&mut self) -> f64 {
        let hi = self.next(26) as i64;
        let lo = self.next(27) as i64;
        ((hi << 27) + lo) as f64 * (1.0 / (1i64 << 53) as f64)
    }
}

struct Jiggle {
    points: Vec<(f64, f64)>,
    start_x: f64,
    start_y: f64,
    default_variation: f64,
    rnd: JavaRandom,
}

impl Jiggle {
    fn new(start_x: f64, start_y: f64, default_variation: f64) -> Self {
        Self::with_random(start_x, start_y, default_variation, JavaRandom::new(424242))
    }

    fn with_random(start_x: f64, start_y: f64, default_variation: f64, rnd: JavaRandom) -> Self {
        Self {
            points: vec![(start_x, start_y)],
            start_x,
            start_y,
            default_variation,
            rnd,
        }
    }

    fn line_to(&mut self, end_x: f64, end_y: f64) {
        let diff_x = (end_x - self.start_x).abs();
        let diff_y = (end_y - self.start_y).abs();
        let distance = (diff_x * diff_x + diff_y * diff_y).sqrt();
        if distance < 0.001 {
            return;
        }

        let mut segments = (distance / 10.0 + 0.5).floor() as i32;
        let mut variation = self.default_variation;
        if segments < 5 {
            segments = 5;
            variation /= 3.0;
        }

        let segments_f = segments as f64;
        let step_x = (end_x - self.start_x).signum() * diff_x / segments_f;
        let step_y = (end_y - self.start_y).signum() * diff_y / segments_f;
        let fx = diff_x / distance;
        let fy = diff_y / distance;

        for s in 0..segments {
            let x = step_x * s as f64 + self.start_x;
            let y = step_y * s as f64 + self.start_y;
            let offset = (self.rnd.next_double() - 0.5) * variation;
            self.points.push((x - offset * fy, y - offset * fx));
        }
        self.points.push((end_x, end_y));
        self.start_x = end_x;
        self.start_y = end_y;
    }

    fn arc_to(&mut self, angle0: f64, angle1: f64, center_x: f64, center_y: f64, rx: f64, ry: f64) {
        let mid = (angle0 + angle1) / 2.0;
        self.line_to(center_x + mid.cos() * rx, center_y + mid.sin() * ry);
        self.line_to(center_x + angle1.cos() * rx, center_y + angle1.sin() * ry);
    }

    fn curve_to(&mut self, curve: Cubic) {
        if curve.flatness() > HAND_CURVE_FLATNESS_MAX
            && curve.endpoint_distance() > HAND_CURVE_MIN_DISTANCE
        {
            let (left, right) = curve.subdivide();
            self.curve_to(left);
            self.curve_to(right);
        } else {
            self.line_to(curve.x2, curve.y2);
        }
    }

    fn points_string(&self, dx: f64, dy: f64) -> String {
        points_string(self.points.iter().map(|&(x, y)| (x + dx, y + dy)))
    }

    fn into_parts(self) -> (Vec<(f64, f64)>, JavaRandom) {
        (self.points, self.rnd)
    }
}

const HAND_PATH_LINE_VARIATION: f64 = 4.0;
const HAND_PATH_CURVE_VARIATION: f64 = 2.0;
const HAND_CURVE_FLATNESS_MAX: f64 = 0.1;
const HAND_CURVE_MIN_DISTANCE: f64 = 20.0;

#[derive(Clone, Copy)]
struct Cubic {
    x1: f64,
    y1: f64,
    ctrl_x1: f64,
    ctrl_y1: f64,
    ctrl_x2: f64,
    ctrl_y2: f64,
    x2: f64,
    y2: f64,
}

impl Cubic {
    fn endpoint_distance(self) -> f64 {
        let dx = self.x2 - self.x1;
        let dy = self.y2 - self.y1;
        (dx * dx + dy * dy).sqrt()
    }

    fn flatness(self) -> f64 {
        self.flatness_sq().sqrt()
    }

    fn flatness_sq(self) -> f64 {
        point_seg_dist_sq(
            self.x1,
            self.y1,
            self.x2,
            self.y2,
            self.ctrl_x1,
            self.ctrl_y1,
        )
        .max(point_seg_dist_sq(
            self.x1,
            self.y1,
            self.x2,
            self.y2,
            self.ctrl_x2,
            self.ctrl_y2,
        ))
    }

    fn subdivide(self) -> (Self, Self) {
        let ctrl_x1 = (self.x1 + self.ctrl_x1) / 2.0;
        let ctrl_y1 = (self.y1 + self.ctrl_y1) / 2.0;
        let ctrl_x2 = (self.x2 + self.ctrl_x2) / 2.0;
        let ctrl_y2 = (self.y2 + self.ctrl_y2) / 2.0;
        let mut center_x = (self.ctrl_x1 + self.ctrl_x2) / 2.0;
        let mut center_y = (self.ctrl_y1 + self.ctrl_y2) / 2.0;
        let ctrl_x12 = (ctrl_x1 + center_x) / 2.0;
        let ctrl_y12 = (ctrl_y1 + center_y) / 2.0;
        let ctrl_x21 = (ctrl_x2 + center_x) / 2.0;
        let ctrl_y21 = (ctrl_y2 + center_y) / 2.0;
        center_x = (ctrl_x12 + ctrl_x21) / 2.0;
        center_y = (ctrl_y12 + ctrl_y21) / 2.0;
        (
            Self {
                x1: self.x1,
                y1: self.y1,
                ctrl_x1,
                ctrl_y1,
                ctrl_x2: ctrl_x12,
                ctrl_y2: ctrl_y12,
                x2: center_x,
                y2: center_y,
            },
            Self {
                x1: center_x,
                y1: center_y,
                ctrl_x1: ctrl_x21,
                ctrl_y1: ctrl_y21,
                ctrl_x2,
                ctrl_y2,
                x2: self.x2,
                y2: self.y2,
            },
        )
    }
}

fn point_seg_dist_sq(x1: f64, y1: f64, x2: f64, y2: f64, px: f64, py: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    if dx == 0.0 && dy == 0.0 {
        let px_dx = px - x1;
        let py_dy = py - y1;
        return px_dx * px_dx + py_dy * py_dy;
    }
    let t = ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy);
    let t = t.clamp(0.0, 1.0);
    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;
    let px_dx = px - proj_x;
    let py_dy = py - proj_y;
    px_dx * px_dx + py_dy * py_dy
}

fn line_points_with_rnd(
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    default_variation: f64,
    rnd: &mut JavaRandom,
) -> Vec<(f64, f64)> {
    let mut points = vec![(start_x, start_y)];
    let diff_x = (end_x - start_x).abs();
    let diff_y = (end_y - start_y).abs();
    let distance = (diff_x * diff_x + diff_y * diff_y).sqrt();
    if distance < 0.001 {
        return points;
    }

    let mut segments = (distance / 10.0 + 0.5).floor() as i32;
    let mut variation = default_variation;
    if segments < 5 {
        segments = 5;
        variation /= 3.0;
    }

    let segments_f = segments as f64;
    let step_x = (end_x - start_x).signum() * diff_x / segments_f;
    let step_y = (end_y - start_y).signum() * diff_y / segments_f;
    let fx = diff_x / distance;
    let fy = diff_y / distance;

    for s in 0..segments {
        let x = step_x * s as f64 + start_x;
        let y = step_y * s as f64 + start_y;
        let offset = (rnd.next_double() - 0.5) * variation;
        points.push((x - offset * fy, y - offset * fx));
    }
    points.push((end_x, end_y));
    points
}

fn points_string(points: impl Iterator<Item = (f64, f64)>) -> String {
    let mut out = String::new();
    for (idx, (x, y)) in points.enumerate() {
        if idx > 0 {
            out.push(',');
        }
        write!(out, "{},{}", pm::fmt_coord(x), pm::fmt_coord(y)).unwrap();
    }
    out
}

pub(crate) fn rect_points(x: f64, y: f64, width: f64, height: f64, rx: f64, ry: f64) -> String {
    let rx = rx.min(width / 2.0);
    let ry = ry.min(height / 2.0);
    let jiggle = if rx == 0.0 && ry == 0.0 {
        let mut jiggle = Jiggle::new(0.0, 0.0, 1.5);
        jiggle.line_to(width, 0.0);
        jiggle.line_to(width, height);
        jiggle.line_to(0.0, height);
        jiggle.line_to(0.0, 0.0);
        jiggle
    } else {
        let mut jiggle = Jiggle::new(rx, 0.0, 1.5);
        jiggle.line_to(width - rx, 0.0);
        jiggle.arc_to(-std::f64::consts::FRAC_PI_2, 0.0, width - rx, ry, rx, ry);
        jiggle.line_to(width, height - ry);
        jiggle.arc_to(
            0.0,
            std::f64::consts::FRAC_PI_2,
            width - rx,
            height - ry,
            rx,
            ry,
        );
        jiggle.line_to(rx, height);
        jiggle.arc_to(
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
            rx,
            height - ry,
            rx,
            ry,
        );
        jiggle.line_to(0.0, ry);
        jiggle.arc_to(
            std::f64::consts::PI,
            3.0 * std::f64::consts::FRAC_PI_2,
            rx,
            ry,
            rx,
            ry,
        );
        jiggle
    };
    jiggle.points_string(x, y)
}

pub(crate) fn line_path(x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    let mut rnd = JavaRandom::new(424242);
    let points = line_points_with_rnd(0.0, 0.0, x2 - x1, y2 - y1, 2.0, &mut rnd);
    let mut out = String::new();
    for (i, (x, y)) in points.iter().enumerate() {
        if i == 0 {
            write!(out, "M{},{}", pm::fmt_coord(x + x1), pm::fmt_coord(y + y1)).unwrap();
        } else {
            write!(out, " L{},{}", pm::fmt_coord(x + x1), pm::fmt_coord(y + y1)).unwrap();
        }
    }
    out
}

pub(crate) fn polygon_points(points: &[(f64, f64)]) -> String {
    let Some(&(x0, y0)) = points.first() else {
        return String::new();
    };
    let mut jiggle = Jiggle::new(x0, y0, 1.5);
    for &(x, y) in &points[1..] {
        jiggle.line_to(x, y);
    }
    jiggle.line_to(x0, y0);
    jiggle.points_string(0.0, 0.0)
}

pub(crate) fn ellipse_points(cx: f64, cy: f64, rx: f64, ry: f64) -> String {
    let mut rnd = JavaRandom::new(424242);
    let mut points = Vec::new();
    let width = 2.0 * rx;
    let height = 2.0 * ry;
    let mut angle = 0.0;
    if (width - height).abs() < f64::EPSILON {
        while angle < std::f64::consts::TAU {
            angle += (10.0 + rnd.next_double() * 10.0).to_radians();
            let variation = 1.0 + (rnd.next_double() - 0.5) / 8.0;
            points.push((
                cx + angle.cos() * width * variation / 2.0,
                cy + angle.sin() * height * variation / 2.0,
            ));
        }
    } else {
        while angle < std::f64::consts::TAU {
            angle += std::f64::consts::PI / 20.0;
            let variation = (rnd.next_double() - 0.5) / 50.0;
            points.push((
                cx + angle.cos() * width / 2.0 + variation * width,
                cy + angle.sin() * height / 2.0 + variation * height,
            ));
        }
    }
    points_string(points.into_iter())
}

fn parse_point_pair(token: &str) -> Option<(f64, f64)> {
    let (x, y) = token.split_once(',')?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

fn parse_command_point(token: &str, cmd: char) -> Option<(f64, f64)> {
    let rest = token.strip_prefix(cmd)?;
    parse_point_pair(rest)
}

pub(crate) fn path_with_rnd(d: &str, rnd: &mut JavaRandom) -> Option<String> {
    let mut out = String::new();
    let mut last: Option<(f64, f64)> = None;
    let tokens: Vec<&str> = d.split_ascii_whitespace().collect();
    let mut idx = 0;
    while idx < tokens.len() {
        let token = tokens[idx];
        let cmd = token.chars().next()?;
        match cmd {
            'M' => {
                let (x, y) = parse_command_point(token, 'M')?;
                if !out.is_empty() {
                    out.push(' ');
                }
                write!(out, "M{},{}", pm::fmt_coord(x), pm::fmt_coord(y)).unwrap();
                last = Some((x, y));
                idx += 1;
            }
            'L' => {
                let (x, y) = parse_command_point(token, 'L')?;
                let (sx, sy) = last?;
                let points = line_points_with_rnd(sx, sy, x, y, HAND_PATH_LINE_VARIATION, rnd);
                for &(px, py) in points.iter().skip(1) {
                    write!(out, " L{},{}", pm::fmt_coord(px), pm::fmt_coord(py)).unwrap();
                }
                last = Some((x, y));
                idx += 1;
            }
            'C' => {
                let (sx, sy) = last?;
                let (ctrl_x1, ctrl_y1) = parse_command_point(token, 'C')?;
                let (ctrl_x2, ctrl_y2) = parse_point_pair(tokens.get(idx + 1).copied()?)?;
                let (x2, y2) = parse_point_pair(tokens.get(idx + 2).copied()?)?;
                let mut jiggle = Jiggle::with_random(sx, sy, HAND_PATH_CURVE_VARIATION, *rnd);
                jiggle.curve_to(Cubic {
                    x1: sx,
                    y1: sy,
                    ctrl_x1,
                    ctrl_y1,
                    ctrl_x2,
                    ctrl_y2,
                    x2,
                    y2,
                });
                let (points, next_rnd) = jiggle.into_parts();
                *rnd = next_rnd;
                for (px, py) in points {
                    write!(out, " L{},{}", pm::fmt_coord(px), pm::fmt_coord(py)).unwrap();
                }
                last = Some((x2, y2));
                idx += 3;
            }
            'A' => {
                // UPathHand turns handwritten arcs into a straight line to
                // the arc endpoint without additional jitter.
                let (x, y) = parse_point_pair(tokens.get(idx + 4).copied()?)?;
                write!(out, " L{},{}", pm::fmt_coord(x), pm::fmt_coord(y)).unwrap();
                last = Some((x, y));
                idx += 5;
            }
            _ => return None,
        }
    }
    Some(out)
}

pub(crate) fn path(d: &str) -> Option<String> {
    let mut rnd = JavaRandom::new(424242);
    path_with_rnd(d, &mut rnd)
}
