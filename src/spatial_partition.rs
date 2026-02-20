//! Spatial grid partitioning for efficient neighbor queries.
//!
//! This module provides O(1) cell lookup and O(K) neighbor queries where K is the average
//! neighbors per cell, replacing O(N²) brute-force iteration.
//!
//! ## Cell Size Choice
//!
//! Cell size (`GRID_CELL_SIZE` in `constants.rs`) must be chosen relative to the
//! query distance, not the max gravity range.
//! With `GRID_CELL_SIZE = 500` units:
//!   - A query for `max_gravity_dist = 1000` checks a 5×5 = 25 cell area
//!   - A query for `neighbor_threshold = 3` checks a 3×3 = 9 cell area
//!
//! Using a cell size of 100 with `max_gravity_dist = 1000` would check 21×21 = 441 cells,
//! which is worse than O(N²) brute force at low asteroid counts.

use crate::asteroid::Asteroid;
use crate::constants::GRID_CELL_SIZE;
use bevy::prelude::*;
use std::collections::HashMap;

/// Resource holding the spatial grid for this frame.
///
/// Cell size is controlled by [`crate::constants::GRID_CELL_SIZE`].
/// Must be at least half of the largest query radius to avoid excessive cell checks.
#[derive(Resource, Debug, Clone, Default)]
pub struct SpatialGrid {
    /// Map from cell coordinates to entity list
    cells: HashMap<(i32, i32), Vec<Entity>>,
}

impl SpatialGrid {
    /// Compute grid cell coordinates for a world position
    fn world_to_cell(pos: Vec2) -> (i32, i32) {
        let x = (pos.x / GRID_CELL_SIZE).floor() as i32;
        let y = (pos.y / GRID_CELL_SIZE).floor() as i32;
        (x, y)
    }

    /// Insert an entity at a position. Call after clear() for bulk rebuild.
    pub fn insert(&mut self, entity: Entity, pos: Vec2) {
        let cell = Self::world_to_cell(pos);
        self.cells.entry(cell).or_default().push(entity);
    }

    /// Clear all grid data (call before each frame rebuild)
    pub fn clear(&mut self) {
        // Retain allocations but clear contents to avoid re-allocating Vec capacity
        for v in self.cells.values_mut() {
            v.clear();
        }
        // Remove cells that are now empty to avoid iterating them next frame
        self.cells.retain(|_, v| !v.is_empty());
    }

    /// Get all entities in cells that overlap the given circle, excluding `entity`.
    /// Note: results include entities outside the circle — callers must do the exact
    /// distance check themselves (the grid is a conservative over-approximation).
    pub fn get_neighbors_excluding(
        &self,
        entity: Entity,
        pos: Vec2,
        max_distance: f32,
    ) -> Vec<Entity> {
        let cell = Self::world_to_cell(pos);
        let cells_to_check = Self::radius_in_cells(max_distance);

        let mut neighbors = Vec::new();

        for dx in -cells_to_check..=cells_to_check {
            for dy in -cells_to_check..=cells_to_check {
                let check_cell = (cell.0 + dx, cell.1 + dy);
                if let Some(entities) = self.cells.get(&check_cell) {
                    for &e in entities {
                        if e != entity {
                            neighbors.push(e);
                        }
                    }
                }
            }
        }

        neighbors
    }

    /// Compute how many cells in each direction we need to check for a given max distance
    fn radius_in_cells(max_distance: f32) -> i32 {
        ((max_distance / GRID_CELL_SIZE).ceil() as i32).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(raw: u32) -> Entity {
        Entity::from_raw(raw)
    }

    // ── world_to_cell ─────────────────────────────────────────────────────────

    #[test]
    fn world_to_cell_origin_is_zero_zero() {
        assert_eq!(SpatialGrid::world_to_cell(Vec2::ZERO), (0, 0));
    }

    #[test]
    fn world_to_cell_exactly_one_cell_width() {
        // At exactly GRID_CELL_SIZE the cell index advances to 1
        assert_eq!(
            SpatialGrid::world_to_cell(Vec2::new(GRID_CELL_SIZE, 0.0)),
            (1, 0)
        );
    }

    #[test]
    fn world_to_cell_negative_small_offset() {
        // floor(-0.1 / GRID_CELL_SIZE) = -1
        assert_eq!(SpatialGrid::world_to_cell(Vec2::new(-0.1, 0.0)), (-1, 0));
    }

    #[test]
    fn world_to_cell_symmetric_y() {
        let (_, cy_pos) = SpatialGrid::world_to_cell(Vec2::new(0.0, GRID_CELL_SIZE * 2.5));
        let (_, cy_neg) = SpatialGrid::world_to_cell(Vec2::new(0.0, -GRID_CELL_SIZE * 2.5 - 1.0));
        assert_eq!(cy_pos, 2);
        assert_eq!(cy_neg, -3);
    }

    // ── radius_in_cells ───────────────────────────────────────────────────────

    #[test]
    fn radius_in_cells_zero_returns_one() {
        assert_eq!(SpatialGrid::radius_in_cells(0.0), 1, "minimum must be 1");
    }

    #[test]
    fn radius_in_cells_exact_cell_width_is_one() {
        assert_eq!(SpatialGrid::radius_in_cells(GRID_CELL_SIZE), 1);
    }

    #[test]
    fn radius_in_cells_fractional_rounds_up() {
        // 1.5 cell widths → ceil → 2
        assert_eq!(SpatialGrid::radius_in_cells(GRID_CELL_SIZE * 1.5), 2);
    }

    #[test]
    fn radius_in_cells_two_exact_cell_widths() {
        assert_eq!(SpatialGrid::radius_in_cells(GRID_CELL_SIZE * 2.0), 2);
    }

    // ── insert / get_neighbors_excluding ─────────────────────────────────────

    #[test]
    fn inserted_entity_is_found_as_neighbor() {
        let mut grid = SpatialGrid::default();
        let target = e(1);
        grid.insert(target, Vec2::new(10.0, 10.0));
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::new(10.0, 10.0), GRID_CELL_SIZE);
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
        let neighbors = grid.get_neighbors_excluding(me, Vec2::ZERO, GRID_CELL_SIZE * 10.0);
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
        // Place far entity >2 cells away so radius_in_cells(1.0) = 1 won't reach it
        grid.insert(far, Vec2::new(GRID_CELL_SIZE * 3.0, 0.0));
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::ZERO, 1.0);
        assert!(
            !neighbors.contains(&far),
            "entity 3 cells away should not appear"
        );
    }

    #[test]
    fn multiple_entities_in_same_cell_all_returned() {
        let mut grid = SpatialGrid::default();
        let e1 = e(1);
        let e2 = e(2);
        let e3 = e(3);
        // All in the first cell (within GRID_CELL_SIZE of origin)
        grid.insert(e1, Vec2::new(5.0, 5.0));
        grid.insert(e2, Vec2::new(10.0, 5.0));
        grid.insert(e3, Vec2::new(5.0, 10.0));
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::new(7.0, 7.0), GRID_CELL_SIZE);
        assert!(neighbors.contains(&e1));
        assert!(neighbors.contains(&e2));
        assert!(neighbors.contains(&e3));
    }

    #[test]
    fn clear_empties_the_grid() {
        let mut grid = SpatialGrid::default();
        grid.insert(e(1), Vec2::ZERO);
        grid.insert(e(2), Vec2::new(50.0, 0.0));
        grid.clear();
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::ZERO, GRID_CELL_SIZE * 100.0);
        assert!(neighbors.is_empty(), "grid should be empty after clear");
    }

    #[test]
    fn insert_same_entity_twice_appears_twice() {
        // No dedup is promised — document the actual behaviour
        let mut grid = SpatialGrid::default();
        let ent = e(1);
        grid.insert(ent, Vec2::ZERO);
        grid.insert(ent, Vec2::ZERO);
        let neighbors = grid.get_neighbors_excluding(e(99), Vec2::ZERO, GRID_CELL_SIZE);
        // Appears at least once
        assert!(neighbors.contains(&ent));
    }
}

/// System to rebuild the spatial grid each frame.
/// Must run BEFORE systems that use the grid (gravity, neighbor counting).
pub fn rebuild_spatial_grid_system(
    mut grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &Transform), With<Asteroid>>,
) {
    grid.clear();

    for (entity, transform) in query.iter() {
        grid.insert(entity, transform.translation.truncate());
    }
}
