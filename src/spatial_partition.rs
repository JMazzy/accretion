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
