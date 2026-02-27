use crate::asteroid::compute_convex_hull_from_points;
use bevy::prelude::*;

/// Returns polygon area via shoelace formula (absolute value).
pub(super) fn polygon_area(v: &[Vec2]) -> f32 {
    let n = v.len();
    if n < 3 {
        return 0.0;
    }
    let mut area2 = 0.0_f32;
    for i in 0..n {
        let j = (i + 1) % n;
        area2 += v[i].x * v[j].y - v[j].x * v[i].y;
    }
    area2.abs() * 0.5
}

/// Split a convex polygon (world-space vertices) with a plane through `origin`
/// whose normal is `axis`.
pub(super) fn split_convex_polygon_world(
    verts: &[Vec2],
    origin: Vec2,
    axis: Vec2,
) -> (Vec<Vec2>, Vec<Vec2>) {
    let mut front: Vec<Vec2> = Vec::new();
    let mut back: Vec<Vec2> = Vec::new();
    let n = verts.len();
    for i in 0..n {
        let a = verts[i];
        let b = verts[(i + 1) % n];
        let da = (a - origin).dot(axis);
        let db = (b - origin).dot(axis);
        if da >= 0.0 {
            front.push(a);
        } else {
            back.push(a);
        }
        if (da > 0.0 && db < 0.0) || (da < 0.0 && db > 0.0) {
            let t = da / (da - db);
            let p = a + (b - a) * t;
            front.push(p);
            back.push(p);
        }
    }
    (front, back)
}

pub(super) fn normalized_fragment_hull(raw: &[Vec2]) -> Option<Vec<Vec2>> {
    if raw.len() < 3 {
        return None;
    }
    let hull = compute_convex_hull_from_points(raw)?;
    if hull.len() < 3 || polygon_area(&hull) <= 1e-4 {
        return None;
    }
    Some(hull)
}

fn closest_point_on_segment(a: Vec2, b: Vec2, p: Vec2) -> Vec2 {
    let ab = b - a;
    let ab_len_sq = ab.length_squared();
    if ab_len_sq <= 1e-8 {
        return a;
    }
    let t = ((p - a).dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    a + ab * t
}

fn closest_point_on_hull(hull: &[Vec2], p: Vec2) -> Option<Vec2> {
    if hull.len() < 2 {
        return None;
    }

    let mut best = None::<(Vec2, f32)>;
    for i in 0..hull.len() {
        let a = hull[i];
        let b = hull[(i + 1) % hull.len()];
        let c = closest_point_on_segment(a, b, p);
        let d2 = c.distance_squared(p);
        match best {
            Some((_, best_d2)) if d2 >= best_d2 => {}
            _ => best = Some((c, d2)),
        }
    }
    best.map(|(point, _)| point)
}

/// Build split parameters so cut lines visually radiate from the impact side.
///
/// - `split_origin`: impact point projected to hull, nudged inward.
/// - `base_normal`: normal perpendicular to the inward ray from impact to centroid.
pub(super) fn impact_radiating_split_basis(
    hull: &[Vec2],
    impact_point: Vec2,
    fallback_axis: Vec2,
) -> Option<(Vec2, Vec2)> {
    if hull.len() < 3 {
        return None;
    }

    let centroid = hull.iter().copied().sum::<Vec2>() / hull.len() as f32;
    let edge_hit = closest_point_on_hull(hull, impact_point).unwrap_or(centroid);

    let mut inward = (centroid - edge_hit).normalize_or_zero();
    if inward == Vec2::ZERO {
        inward = fallback_axis.normalize_or_zero();
    }
    if inward == Vec2::ZERO {
        inward = Vec2::Y;
    }

    let split_origin = edge_hit + inward * 1.5;
    let mut base_normal = Vec2::new(-inward.y, inward.x).normalize_or_zero();
    if base_normal == Vec2::ZERO {
        base_normal = fallback_axis.normalize_or_zero();
    }
    if base_normal == Vec2::ZERO {
        base_normal = Vec2::X;
    }

    Some((split_origin, base_normal))
}

pub(super) fn even_mass_partition(total_mass: u32, piece_count: usize) -> Vec<u32> {
    if piece_count == 0 {
        return Vec::new();
    }
    let pieces = piece_count as u32;
    let base = total_mass / pieces;
    let remainder = total_mass % pieces;
    let mut masses = vec![base; piece_count];
    for mass in masses.iter_mut().take(remainder as usize) {
        *mass += 1;
    }
    masses
}

pub(super) fn area_weighted_mass_partition(
    areas: &[f32],
    total_mass: u32,
    piece_count: usize,
) -> Vec<u32> {
    if piece_count == 0 {
        return Vec::new();
    }
    if total_mass <= piece_count as u32 {
        return vec![1; piece_count];
    }

    let safe_areas: Vec<f32> = if areas.len() == piece_count {
        areas.iter().map(|a| a.max(1e-4)).collect()
    } else {
        vec![1.0; piece_count]
    };

    let mut masses = vec![1_u32; piece_count];
    let remaining = total_mass - piece_count as u32;
    let area_sum = safe_areas.iter().sum::<f32>().max(1e-4);

    let mut used = 0_u32;
    let mut fractional: Vec<(usize, f32)> = Vec::with_capacity(piece_count);
    for (idx, area) in safe_areas.iter().enumerate() {
        let exact = remaining as f32 * (*area / area_sum);
        let whole = exact.floor() as u32;
        masses[idx] += whole;
        used += whole;
        fractional.push((idx, exact - whole as f32));
    }

    let mut leftovers = remaining.saturating_sub(used) as usize;
    fractional.sort_by(|(i_a, frac_a), (i_b, frac_b)| {
        frac_b.total_cmp(frac_a).then_with(|| i_a.cmp(i_b))
    });
    for (idx, _) in fractional {
        if leftovers == 0 {
            break;
        }
        masses[idx] += 1;
        leftovers -= 1;
    }

    masses
}
