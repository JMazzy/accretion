//! Spatial grid partitioning for efficient neighbor queries.
//!
//! This module provides O(1) cell lookup and O(K) neighbor queries where K is the average
//! neighbors per cell, replacing O(N²) brute-force iteration.
//!
//! Grid cell size is set to 100 units, ensuring asteroids within max_gravity_dist (1000 units)
//! are always in adjacent cells.

use bevy::prelude::*;
use std::collections::HashMap;

/// Size of each grid cell in world units
const GRID_CELL_SIZE: f32 = 100.0;

/// Resource holding the spatial grid for this frame
#[derive(Resource, Debug, Clone, Default)]
pub struct SpatialGrid {
    /// Map from cell coordinates to entity list
    cells: HashMap<(i32, i32), Vec<Entity>>,
    /// Map from entity to its current cell
    entity_cells: HashMap<Entity, (i32, i32)>,
}

impl SpatialGrid {
    /// Compute grid cell coordinates for a world position
    fn world_to_cell(pos: Vec2) -> (i32, i32) {
        let x = (pos.x / GRID_CELL_SIZE).floor() as i32;
        let y = (pos.y / GRID_CELL_SIZE).floor() as i32;
        (x, y)
    }

    /// Insert or update an entity's position in the grid
    pub fn insert(&mut self, entity: Entity, pos: Vec2) {
        let cell = Self::world_to_cell(pos);

        // Remove from old cell if exists
        if let Some(old_cell) = self.entity_cells.get(&entity) {
            if let Some(cell_entities) = self.cells.get_mut(old_cell) {
                cell_entities.retain(|e| e != &entity);
            }
        }

        // Add to new cell
        self.cells.entry(cell).or_default().push(entity);
        self.entity_cells.insert(entity, cell);
    }

    /// Clear all grid data (called each frame before rebuild)
    pub fn clear(&mut self) {
        self.cells.clear();
        self.entity_cells.clear();
    }

    /// Get all neighbors within a given distance from a position
    /// Returns entities in cells that could contain neighbors within the distance
    pub fn get_neighbors(&self, pos: Vec2, max_distance: f32) -> Vec<Entity> {
        let cell = Self::world_to_cell(pos);
        let cells_to_check = Self::radius_in_cells(max_distance);

        let mut neighbors = Vec::new();

        for dx in -cells_to_check..=cells_to_check {
            for dy in -cells_to_check..=cells_to_check {
                let check_cell = (cell.0 + dx, cell.1 + dy);
                if let Some(entities) = self.cells.get(&check_cell) {
                    neighbors.extend(entities.iter().copied());
                }
            }
        }

        neighbors
    }

    /// Get all neighbors within a given distance, filtering out the query entity itself
    pub fn get_neighbors_excluding(
        &self,
        entity: Entity,
        pos: Vec2,
        max_distance: f32,
    ) -> Vec<Entity> {
        self.get_neighbors(pos, max_distance)
            .into_iter()
            .filter(|e| e != &entity)
            .collect()
    }

    /// Compute how many cells in each direction we need to check for a given max distance
    fn radius_in_cells(max_distance: f32) -> i32 {
        ((max_distance / GRID_CELL_SIZE).ceil() as i32).max(1)
    }
}

/// System to rebuild the spatial grid each frame
/// Must run BEFORE systems that use the grid (gravity, neighbor counting)
pub fn rebuild_spatial_grid_system(
    mut grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &Transform), With<Asteroid>>,
) {
    grid.clear();

    for (entity, transform) in query.iter() {
        let pos = transform.translation.truncate();
        grid.insert(entity, pos);
    }
}

// Re-export Asteroid for use in this module
use crate::asteroid::Asteroid;
