//! Flat-arena KD-tree spatial index for efficient neighbour queries.
//!
//! Replaces the previous pointer-based KD-tree implementation.  The previous
//! version allocated one `Box<KdNode>` **per asteroid per frame**, creating
//! enormous allocator pressure at 60 fps (100 asteroids × 60 fps = 6 000
//! heap allocs/sec just for the spatial index).
//!
//! ## Implementation
//!
//! All nodes are stored in a single `Vec<KdFlat>` that is *cleared* (but not
//! freed) each frame, so the backing allocation grows once and then stays
//! stable.  Child pointers are compact `u32` indices into that Vec;
//! `NULL_IDX = u32::MAX` signals a missing child.
//!
//! Build cost: O(N log N) — one sort pass per level of the tree.
//! Query cost: O(K + log N) — exact Euclidean sphere test with subtree pruning.
//! Zero per-frame heap allocations after the first frame.
//!
//! ## API compatibility
//!
//! The public API (`SpatialGrid`, `get_neighbors_excluding`,
//! `rebuild_spatial_grid_system`) is identical to the old grid implementation
//! so all call-sites require no changes.

use crate::asteroid::Asteroid;
use bevy::prelude::*;
use std::cmp::Ordering;

// ── Flat KD-tree node ─────────────────────────────────────────────────────────

const NULL_IDX: u32 = u32::MAX;

/// One node in the flat-arena KD-tree.
#[derive(Clone)]
struct KdFlat {
    entity: Entity,
    pos: Vec2,
    left: u32,  // index into SpatialGrid::nodes, or NULL_IDX
    right: u32, // index into SpatialGrid::nodes, or NULL_IDX
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Flat-arena KD-tree spatial index.  Rebuilt fresh each frame from current
/// asteroid positions.  The `nodes` Vec is reused across frames — it grows to
/// fit the largest N seen and is never freed, so per-frame cost is zero extra
/// heap allocations after warm-up.
#[derive(Resource)]
pub struct SpatialGrid {
    /// Flat node storage.  Rebuilt each frame via `rebuild`.
    nodes: Vec<KdFlat>,
    /// Root node index, or `NULL_IDX` when the tree is empty.
    root: u32,
    /// Pending inserts for the `insert` / `build` API (used by tests).
    pending: Vec<(Entity, Vec2)>,
    /// Scratch sort-buffer reused by `rebuild_spatial_grid_system` so the ECS
    /// system can fill it by extending and then call `rebuild_in_place` without
    /// any per-tick heap allocation.
    pub pts_scratch: Vec<(Entity, Vec2)>,
}

impl Default for SpatialGrid {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            root: NULL_IDX,
            pending: Vec::new(),
            pts_scratch: Vec::new(),
        }
    }
}

impl std::fmt::Debug for SpatialGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpatialGrid")
            .field("node_count", &self.nodes.len())
            .field("has_tree", &(self.root != NULL_IDX))
            .finish()
    }
}

impl SpatialGrid {
    // ── Bulk API (used by rebuild_spatial_grid_system) ────────────────────────

    /// Rebuild the index in one step from the provided points.
    ///
    /// Clears `nodes` (retaining allocation) then builds a balanced KD-tree.
    pub fn rebuild(&mut self, points: Vec<(Entity, Vec2)>) {
        self.pts_scratch.clear();
        self.pts_scratch.extend(points);
        self.rebuild_in_place();
    }

    /// Rebuild from `pts_scratch` in place.  The caller fills `pts_scratch`;
    /// this method sorts it and constructs the balanced KD-tree with zero extra
    /// heap allocations.
    pub fn rebuild_in_place(&mut self) {
        self.nodes.clear();
        self.pending.clear();
        let n = self.pts_scratch.len();
        if n == 0 {
            self.root = NULL_IDX;
            return;
        }
        if self.nodes.capacity() < n {
            self.nodes.reserve(n - self.nodes.len());
        }
        self.root = Self::build_recursive(&mut self.nodes, &mut self.pts_scratch, 0);
    }

    /// Recursive median-split builder.  Returns the index of the created node.
    fn build_recursive(nodes: &mut Vec<KdFlat>, pts: &mut [(Entity, Vec2)], depth: usize) -> u32 {
        if pts.is_empty() {
            return NULL_IDX;
        }

        let axis = depth & 1; // 0 → X, 1 → Y
        pts.sort_unstable_by(|a, b| {
            let av = if axis == 0 { a.1.x } else { a.1.y };
            let bv = if axis == 0 { b.1.x } else { b.1.y };
            av.partial_cmp(&bv).unwrap_or(Ordering::Equal)
        });

        let mid = pts.len() / 2;
        let (entity, pos) = pts[mid];

        // Reserve the slot for this node before recursing so the index is known.
        let idx = nodes.len() as u32;
        nodes.push(KdFlat {
            entity,
            pos,
            left: NULL_IDX,
            right: NULL_IDX,
        });

        let left = Self::build_recursive(nodes, &mut pts[..mid], depth + 1);
        let right = Self::build_recursive(nodes, &mut pts[mid + 1..], depth + 1);
        nodes[idx as usize].left = left;
        nodes[idx as usize].right = right;

        idx
    }

    // ── Debug visualization API ─────────────────────────────────────────────

    /// Collect KD-tree split lines for debug rendering.
    ///
    /// `min`/`max` define the world-space bounding rectangle used as the root
    /// region for recursive KD split visualization.
    pub fn collect_debug_split_lines(&self, min: Vec2, max: Vec2, out: &mut Vec<(Vec2, Vec2)>) {
        out.clear();
        if self.root == NULL_IDX {
            return;
        }
        self.collect_debug_split_lines_recursive(self.root, min, max, 0, out);
    }

    fn collect_debug_split_lines_recursive(
        &self,
        idx: u32,
        min: Vec2,
        max: Vec2,
        depth: usize,
        out: &mut Vec<(Vec2, Vec2)>,
    ) {
        if idx == NULL_IDX {
            return;
        }

        let node = &self.nodes[idx as usize];
        let axis = depth & 1;

        if axis == 0 {
            let x = node.pos.x;
            out.push((Vec2::new(x, min.y), Vec2::new(x, max.y)));
            self.collect_debug_split_lines_recursive(
                node.left,
                min,
                Vec2::new(x, max.y),
                depth + 1,
                out,
            );
            self.collect_debug_split_lines_recursive(
                node.right,
                Vec2::new(x, min.y),
                max,
                depth + 1,
                out,
            );
        } else {
            let y = node.pos.y;
            out.push((Vec2::new(min.x, y), Vec2::new(max.x, y)));
            self.collect_debug_split_lines_recursive(
                node.left,
                min,
                Vec2::new(max.x, y),
                depth + 1,
                out,
            );
            self.collect_debug_split_lines_recursive(
                node.right,
                Vec2::new(min.x, y),
                max,
                depth + 1,
                out,
            );
        }
    }

    // ── Insert / build API (used by tests only) ───────────────────────────────

    /// Clear the index, ready for a new set of inserts.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.pending.clear();
        self.root = NULL_IDX;
    }

    /// Stage an entity at `pos` for the next [`build`](Self::build) call.
    #[allow(dead_code)]
    pub fn insert(&mut self, entity: Entity, pos: Vec2) {
        self.pending.push((entity, pos));
    }

    /// Build the KD-tree from all staged `insert` calls, then clear the buffer.
    #[allow(dead_code)]
    pub fn build(&mut self) {
        let pts = std::mem::take(&mut self.pending);
        self.rebuild(pts);
    }

    // ── Query API ─────────────────────────────────────────────────────────────

    /// Return all entities within `max_distance` of `pos`, **excluding** `entity`.
    ///
    /// Uses an exact Euclidean sphere test; no caller-side re-filtering needed.
    ///
    /// Prefer [`query_neighbors_into`](Self::query_neighbors_into) in hot paths
    /// to avoid the per-call `Vec` allocation.
    #[allow(dead_code)] // convenience wrapper used by unit tests
    pub fn get_neighbors_excluding(
        &self,
        entity: Entity,
        pos: Vec2,
        max_distance: f32,
    ) -> Vec<Entity> {
        let mut results = Vec::new();
        self.query_neighbors_into(entity, pos, max_distance, &mut results);
        results
    }

    /// Like [`get_neighbors_excluding`](Self::get_neighbors_excluding) but fills
    /// a caller-supplied buffer instead of allocating a new `Vec`.
    ///
    /// `out` is cleared before being filled, so the same buffer can be reused
    /// across calls to eliminate per-query heap allocations entirely.
    pub fn query_neighbors_into(
        &self,
        entity: Entity,
        pos: Vec2,
        max_distance: f32,
        out: &mut Vec<Entity>,
    ) {
        out.clear();
        if self.root != NULL_IDX {
            self.query_radius(self.root, pos, max_distance * max_distance, entity, 0, out);
        }
    }

    /// Recursive KD-tree sphere query.
    fn query_radius(
        &self,
        idx: u32,
        center: Vec2,
        radius_sq: f32,
        exclude: Entity,
        depth: usize,
        results: &mut Vec<Entity>,
    ) {
        if idx == NULL_IDX {
            return;
        }
        let node = &self.nodes[idx as usize];
        let diff = node.pos - center;

        // Include this node if in range and not excluded.
        if diff.length_squared() <= radius_sq && node.entity != exclude {
            results.push(node.entity);
        }

        // Split-plane signed distance (positive → node is right/above center).
        let axis = depth & 1;
        let split_dist = if axis == 0 { diff.x } else { diff.y };
        let split_dist_sq = split_dist * split_dist;

        let (near, far) = if split_dist >= 0.0 {
            (node.left, node.right)
        } else {
            (node.right, node.left)
        };

        self.query_radius(near, center, radius_sq, exclude, depth + 1, results);
        if split_dist_sq <= radius_sq {
            self.query_radius(far, center, radius_sq, exclude, depth + 1, results);
        }
    }
}

// ── ECS rebuild system ────────────────────────────────────────────────────────

/// Rebuild the KD-tree from current asteroid positions each physics tick.
///
/// Scheduled in `FixedUpdate` just before `nbody_gravity_system`.  Uses the
/// pre-allocated `pts_scratch` buffer in `SpatialGrid` to avoid any per-tick
/// heap allocation.
pub fn rebuild_spatial_grid_system(
    mut grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &Transform), With<Asteroid>>,
) {
    grid.pts_scratch.clear();
    for (entity, transform) in query.iter() {
        grid.pts_scratch
            .push((entity, transform.translation.truncate()));
    }
    grid.rebuild_in_place();
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn e(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).expect("test entity index must not be u32::MAX")
    }

    #[test]
    fn inserted_entity_is_found_as_neighbor() {
        let mut grid = SpatialGrid::default();
        let target = e(1);
        grid.insert(target, Vec2::new(10.0, 10.0));
        grid.build();
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::new(10.0, 10.0), 500.0);
        assert!(
            neighbors.contains(&target),
            "inserted entity should appear as neighbor"
        );
    }

    #[test]
    fn get_neighbors_never_includes_self() {
        let mut grid = SpatialGrid::default();
        let me = e(1);
        grid.insert(me, Vec2::ZERO);
        grid.build();
        let neighbors = grid.get_neighbors_excluding(me, Vec2::ZERO, 10_000.0);
        assert!(
            !neighbors.contains(&me),
            "entity should not appear in its own neighbor list"
        );
    }

    #[test]
    fn entity_far_outside_radius_not_returned() {
        let mut grid = SpatialGrid::default();
        let near = e(1);
        let far = e(2);
        grid.insert(near, Vec2::ZERO);
        grid.insert(far, Vec2::new(3000.0, 0.0));
        grid.build();
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::ZERO, 1.0);
        assert!(
            !neighbors.contains(&far),
            "entity 3000 units away should not appear in a radius-1 query"
        );
    }

    #[test]
    fn multiple_entities_in_same_region_all_returned() {
        let mut grid = SpatialGrid::default();
        let e1 = e(1);
        let e2 = e(2);
        let e3 = e(3);
        grid.insert(e1, Vec2::new(5.0, 5.0));
        grid.insert(e2, Vec2::new(10.0, 5.0));
        grid.insert(e3, Vec2::new(5.0, 10.0));
        grid.build();
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::new(7.0, 7.0), 20.0);
        assert!(neighbors.contains(&e1));
        assert!(neighbors.contains(&e2));
        assert!(neighbors.contains(&e3));
    }

    #[test]
    fn clear_empties_the_index() {
        let mut grid = SpatialGrid::default();
        grid.insert(e(1), Vec2::ZERO);
        grid.insert(e(2), Vec2::new(50.0, 0.0));
        grid.build();
        grid.clear();
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::ZERO, 100_000.0);
        assert!(neighbors.is_empty(), "index should be empty after clear");
    }

    #[test]
    fn insert_same_entity_twice_appears_at_least_once() {
        let mut grid = SpatialGrid::default();
        let ent = e(1);
        grid.insert(ent, Vec2::ZERO);
        grid.insert(ent, Vec2::ZERO);
        grid.build();
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::ZERO, 500.0);
        assert!(neighbors.contains(&ent));
    }

    #[test]
    fn entity_exactly_at_max_distance_is_included() {
        let mut grid = SpatialGrid::default();
        let target = e(1);
        grid.insert(target, Vec2::new(100.0, 0.0));
        grid.build();
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::ZERO, 100.0);
        assert!(
            neighbors.contains(&target),
            "entity exactly at radius should be included"
        );
    }

    #[test]
    fn zero_radius_returns_only_entities_at_same_position() {
        let mut grid = SpatialGrid::default();
        let at_origin = e(1);
        let elsewhere = e(2);
        grid.insert(at_origin, Vec2::ZERO);
        grid.insert(elsewhere, Vec2::new(1.0, 0.0));
        grid.build();
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::ZERO, 0.0);
        assert!(neighbors.contains(&at_origin));
        assert!(!neighbors.contains(&elsewhere));
    }

    #[test]
    fn rebuild_replaces_previous_contents() {
        let mut grid = SpatialGrid::default();
        let old = e(1);
        grid.rebuild(vec![(old, Vec2::new(500.0, 0.0))]);
        assert!(grid
            .get_neighbors_excluding(e(99), Vec2::new(500.0, 0.0), 10.0)
            .contains(&old));

        let new_e = e(2);
        grid.rebuild(vec![(new_e, Vec2::ZERO)]);
        assert!(!grid
            .get_neighbors_excluding(e(99), Vec2::new(500.0, 0.0), 10.0)
            .contains(&old));
        assert!(grid
            .get_neighbors_excluding(e(99), Vec2::ZERO, 10.0)
            .contains(&new_e));
    }

    #[test]
    fn all_close_entities_found_in_large_set() {
        let mut points: Vec<(Entity, Vec2)> = (0u32..200)
            .map(|i| {
                let angle = i as f32 * 0.17;
                let r = 50.0 + (i as f32) * 4.5;
                (e(i + 1), Vec2::new(r * angle.cos(), r * angle.sin()))
            })
            .collect();
        let close: Vec<Entity> = (201u32..206).map(e).collect();
        for (idx, &ent) in close.iter().enumerate() {
            points.push((ent, Vec2::new(idx as f32 * 0.5, 0.0)));
        }

        let mut grid = SpatialGrid::default();
        grid.rebuild(points);

        let results = grid.get_neighbors_excluding(e(9999), Vec2::ZERO, 3.0);
        for &ent in &close {
            assert!(results.contains(&ent), "close entity {:?} not found", ent);
        }
        for i in 0u32..200 {
            assert!(
                !results.contains(&e(i + 1)),
                "far entity e({}) should not appear",
                i + 1
            );
        }
    }
}
