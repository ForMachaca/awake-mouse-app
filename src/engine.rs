use std::f64::consts::PI;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::platform::{NativeBackend, Point, Rect};

#[derive(Clone, Copy)]
pub struct MotionConfig {
    pub interval: Duration,
    pub travel_px: f64,
    pub duration: Duration,
    pub return_to_origin: bool,
}

pub fn perform_cycle<B: NativeBackend>(
    backend: &mut B,
    config: MotionConfig,
) -> Result<String, String> {
    let origin = backend.cursor_position()?;
    let bounds = backend.screen_bounds()?;
    let mut rng = RandomSource::from_entropy(origin);
    let points = build_path(origin, bounds, config, &mut rng);
    let final_point = points.last().copied().unwrap_or(origin);
    let step_delay = step_delay(config.duration, points.len());

    for point in points {
        backend.move_cursor(point)?;
        thread::sleep(step_delay);
    }

    let end = if config.return_to_origin {
        "并返回原点"
    } else {
        "并停留在目标点"
    };

    Ok(format!(
        "已执行一次随机原生鼠标轨迹，位移约 {:.0}px，{}。",
        estimate_displacement(origin, final_point, config),
        end
    ))
}

fn build_path(
    origin: Point,
    bounds: Rect,
    config: MotionConfig,
    rng: &mut RandomSource,
) -> Vec<Point> {
    let travel = config.travel_px.clamp(2.0, 120.0);
    let target = choose_target(origin, bounds, travel, rng);
    let normal = perpendicular(origin, target);
    let distance = origin.distance_to(target).max(1.0);
    let curve_height = distance * rng.range_f64(0.20, 0.52) * rng.sign();
    let skew = distance * rng.range_f64(-0.18, 0.18);
    let control = midpoint(origin, target)
        .offset(normal.x * curve_height, normal.y * curve_height)
        .offset(skew, -skew * 0.35);
    let mut points = sample_curve(origin, control, target, rng.range_usize(14, 24));

    if config.return_to_origin {
        let return_control = midpoint(target, origin).offset(
            -normal.x * curve_height * rng.range_f64(0.58, 0.94),
            -normal.y * curve_height * rng.range_f64(0.58, 0.94),
        );
        let mut back = sample_curve(target, return_control, origin, rng.range_usize(12, 21));
        if !back.is_empty() {
            back.remove(0);
        }
        points.extend(back);
    }

    dedupe(points)
}

fn choose_target(origin: Point, bounds: Rect, travel: f64, rng: &mut RandomSource) -> Point {
    let right_room = (bounds.max_x() - origin.x - 3.0).max(0.0);
    let left_room = (origin.x - bounds.min_x - 3.0).max(0.0);
    let down_room = (bounds.max_y() - origin.y - 3.0).max(0.0);
    let up_room = (origin.y - bounds.min_y - 3.0).max(0.0);

    let horizontal_bias = if right_room >= left_room { 0.62 } else { 0.38 };
    let vertical_bias = if down_room >= up_room { 0.58 } else { 0.42 };
    let dx_sign = if rng.next_unit() <= horizontal_bias {
        1.0
    } else {
        -1.0
    };
    let dy_sign = if rng.next_unit() <= vertical_bias {
        1.0
    } else {
        -1.0
    };

    let dx_limit = if dx_sign > 0.0 { right_room } else { left_room };
    let dy_limit = if dy_sign > 0.0 { down_room } else { up_room };
    let dx = travel.min(dx_limit).max(1.0) * rng.range_f64(0.55, 1.05);
    let dy = (travel * rng.range_f64(0.22, 0.78)).min(dy_limit).max(1.0);

    Point::new(
        clamp(
            origin.x + dx_sign * dx,
            bounds.min_x + 2.0,
            bounds.max_x() - 2.0,
        ),
        clamp(
            origin.y + dy_sign * dy,
            bounds.min_y + 2.0,
            bounds.max_y() - 2.0,
        ),
    )
}

fn sample_curve(start: Point, control: Point, end: Point, steps: usize) -> Vec<Point> {
    let mut points = Vec::with_capacity(steps + 1);

    for index in 0..=steps {
        let t = index as f64 / steps as f64;
        let eased = ease_in_out_sine(t);
        points.push(quadratic_bezier(start, control, end, eased));
    }

    points
}

fn quadratic_bezier(start: Point, control: Point, end: Point, t: f64) -> Point {
    let inv = 1.0 - t;
    let x = inv * inv * start.x + 2.0 * inv * t * control.x + t * t * end.x;
    let y = inv * inv * start.y + 2.0 * inv * t * control.y + t * t * end.y;
    Point::new(x, y)
}

fn midpoint(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

fn perpendicular(a: Point, b: Point) -> Point {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length = (dx * dx + dy * dy).sqrt().max(1.0);
    Point::new(-dy / length, dx / length)
}

fn dedupe(points: Vec<Point>) -> Vec<Point> {
    let mut deduped = Vec::with_capacity(points.len());

    for point in points {
        let should_push = deduped
            .last()
            .map(|last: &Point| last.distance_to(point) >= 0.5)
            .unwrap_or(true);

        if should_push {
            deduped.push(point);
        }
    }

    deduped
}

fn ease_in_out_sine(t: f64) -> f64 {
    -(PI * t).cos() * 0.5 + 0.5
}

fn step_delay(duration: Duration, steps: usize) -> Duration {
    let millis = duration.as_millis() as u64;
    let divisor = steps.max(1) as u64;
    Duration::from_millis((millis / divisor).max(8))
}

fn estimate_displacement(origin: Point, end: Point, config: MotionConfig) -> f64 {
    if config.return_to_origin {
        config.travel_px
    } else {
        origin.distance_to(end).max(1.0)
    }
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

struct RandomSource {
    state: u64,
}

impl RandomSource {
    fn from_entropy(origin: Point) -> Self {
        let time_seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let point_seed = origin.x.to_bits() ^ origin.y.to_bits().rotate_left(17);
        let state = time_seed ^ point_seed ^ 0xA076_1D64_78BD_642F;
        Self {
            state: state.max(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn next_unit(&mut self) -> f64 {
        let value = self.next_u64() >> 11;
        value as f64 / ((1u64 << 53) as f64)
    }

    fn range_f64(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.next_unit()
    }

    fn range_usize(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        min + (self.next_u64() as usize % (max - min + 1))
    }

    fn sign(&mut self) -> f64 {
        if self.next_u64() & 1 == 0 {
            1.0
        } else {
            -1.0
        }
    }
}
