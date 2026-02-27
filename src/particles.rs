//! Particle effects: impact sparks, missile trails, debris dust, and merge glows.
//!
//! ## Design
//!
//! Particles are lightweight ECS entities with a [`Particle`] component that
//! stores physics state (velocity, age, colour).  A two-system pipeline handles
//! them:
//!
//! | System                     | Schedule | Purpose                                    |
//! |----------------------------|----------|--------------------------------------------|
//! | `attach_particle_mesh_system` | Update | Attach `Mesh2d` to freshly-spawned particles |
//! | `particle_update_system`   | Update   | Move, fade, and despawn expired particles  |
//!
//! Particle entities are spawned by free functions (`spawn_impact_particles`,
//! `spawn_missile_trail_particles`, `spawn_debris_particles`, `spawn_merge_particles`) that take only
//! `&mut Commands` — no `Assets` access needed at spawn time.  The
//! `attach_particle_mesh_system` supplies the Mesh2d one frame later, which is
//! imperceptible at 60 Hz.
//!
//! A single shared circle-mesh [`ParticleMesh`] resource is created at plugin
//! startup to avoid per-particle mesh allocation.  Each particle receives its
//! own unique [`ColorMaterial`] so its alpha can be faded individually.

use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_mesh::{Indices, PrimitiveTopology};
use rand::Rng;

// ── Resources ────────────────────────────────────────────────────────────────

/// Shared circle mesh used by all particle entities (created once at startup).
#[derive(Resource)]
pub struct ParticleMesh(pub Handle<Mesh>);

// ── Component ────────────────────────────────────────────────────────────────

/// Short-lived visual particle entity.
///
/// After spawning, `attach_particle_mesh_system` inserts the `Mesh2d` /
/// `MeshMaterial2d` pair and writes the material handle into `material`.
/// `particle_update_system` then moves, fades, and eventually despawns it.
#[derive(Component)]
pub struct Particle {
    /// World-space velocity (units/s).
    pub velocity: Vec2,
    /// Time alive so far (s).
    pub age: f32,
    /// Total lifetime (s); entity is despawned when `age >= lifetime`.
    pub lifetime: f32,
    /// Base colour red channel (sRGB, 0–1).
    pub r: f32,
    /// Base colour green channel.
    pub g: f32,
    /// Base colour blue channel.
    pub b: f32,
    /// Handle to this particle's unique `ColorMaterial` so `particle_update_system`
    /// can update the alpha.  `None` until `attach_particle_mesh_system` runs.
    pub material: Option<Handle<ColorMaterial>>,
}

/// Visual mode for tractor beam particle emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TractorBeamVfxMode {
    Pull,
    Push,
    Freeze,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_particle_mesh).add_systems(
            Update,
            (attach_particle_mesh_system, particle_update_system).chain(),
        );
    }
}

// ── Startup system ────────────────────────────────────────────────────────────

/// Create the shared circle mesh and store it as a [`ParticleMesh`] resource.
fn init_particle_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let handle = meshes.add(circle_mesh(2.0, 6));
    commands.insert_resource(ParticleMesh(handle));
}

// ── Update systems ────────────────────────────────────────────────────────────

/// Attach `Mesh2d` + `MeshMaterial2d` to every newly-spawned [`Particle`].
///
/// Uses [`Added<Particle>`] so it only runs for particles that appeared since
/// the last frame — zero overhead for the steady-state particle population.
pub fn attach_particle_mesh_system(
    mut commands: Commands,
    particle_mesh: Res<ParticleMesh>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<(Entity, &mut Particle), Added<Particle>>,
) {
    for (entity, mut particle) in query.iter_mut() {
        let mat_handle = materials.add(ColorMaterial::from_color(Color::srgba(
            particle.r, particle.g, particle.b, 1.0,
        )));
        particle.material = Some(mat_handle.clone());
        commands
            .entity(entity)
            .insert((Mesh2d(particle_mesh.0.clone()), MeshMaterial2d(mat_handle)));
    }
}

/// Advance all particles: translate by velocity, fade alpha quadratically,
/// and despawn any whose age has exceeded their lifetime.
pub fn particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<(Entity, &mut Transform, &mut Particle)>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.age += dt;

        if particle.age >= particle.lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // Translate by velocity.
        transform.translation.x += particle.velocity.x * dt;
        transform.translation.y += particle.velocity.y * dt;

        // Quadratic ease-out alpha: bright at birth, rapid fade at end.
        let t = particle.age / particle.lifetime; // 0 → 1
        let alpha = (1.0 - t).powi(2);

        if let Some(ref handle) = particle.material {
            if let Some(mat) = materials.get_mut(handle) {
                mat.color = Color::srgba(particle.r, particle.g, particle.b, alpha);
            }
        }
    }
}

// ── Public spawn helpers ──────────────────────────────────────────────────────

/// Spawn impact sparks at `pos` when a projectile hits an asteroid.
///
/// `asteroid_vel` is added to each particle's base velocity so the sparks
/// appear to eject from the hit surface rather than the world origin.
/// `impact_dir` is the normalised direction from projectile toward asteroid.
pub fn spawn_impact_particles(
    commands: &mut Commands,
    pos: Vec2,
    impact_dir: Vec2,
    asteroid_vel: Vec2,
) {
    let mut rng = rand::thread_rng();
    let count = 8_u32;

    for _ in 0..count {
        // Fan outward from the impact direction with ±70° spread.
        let base_angle = impact_dir.y.atan2(impact_dir.x);
        let spread = std::f32::consts::FRAC_PI_2 * 0.78; // ±70°
        let angle = base_angle + rng.gen_range(-spread..spread);
        let speed = rng.gen_range(60.0_f32..160.0_f32);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed + asteroid_vel * 0.3;

        // Orange-yellow sparks with slight variation.
        let r = rng.gen_range(0.90_f32..1.0_f32);
        let g = rng.gen_range(0.50_f32..0.75_f32);
        let b = rng.gen_range(0.0_f32..0.20_f32);

        let lifetime = rng.gen_range(0.20_f32..0.40_f32);
        let offset = Vec2::new(rng.gen_range(-2.0..2.0), rng.gen_range(-2.0..2.0));

        commands.spawn((
            Particle {
                velocity,
                age: 0.0,
                lifetime,
                r,
                g,
                b,
                material: None,
            },
            Transform::from_translation((pos + offset).extend(0.9)),
            Visibility::default(),
        ));
    }
}

/// Spawn a short missile exhaust burst opposite to the missile's movement.
///
/// `reverse_dir` is expected to point opposite the missile velocity direction.
pub fn spawn_missile_trail_particles(
    commands: &mut Commands,
    pos: Vec2,
    reverse_dir: Vec2,
    missile_vel: Vec2,
) {
    let mut rng = rand::thread_rng();
    let count = 2_u32;

    let base = if reverse_dir.length_squared() > 1e-6 {
        reverse_dir.normalize()
    } else {
        Vec2::NEG_Y
    };
    let base_angle = base.y.atan2(base.x);

    for _ in 0..count {
        let angle = base_angle + rng.gen_range(-0.35_f32..0.35_f32);
        let speed = rng.gen_range(20.0_f32..65.0_f32);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed + missile_vel * 0.10;

        let r = rng.gen_range(0.95_f32..1.0_f32);
        let g = rng.gen_range(0.45_f32..0.68_f32);
        let b = rng.gen_range(0.05_f32..0.20_f32);

        let lifetime = rng.gen_range(0.10_f32..0.22_f32);
        let lateral = Vec2::new(-base.y, base.x) * rng.gen_range(-1.1_f32..1.1_f32);
        let back_offset = base * rng.gen_range(0.0_f32..2.5_f32);

        commands.spawn((
            Particle {
                velocity,
                age: 0.0,
                lifetime,
                r,
                g,
                b,
                material: None,
            },
            Transform::from_translation((pos + lateral + back_offset).extend(0.9)),
            Visibility::default(),
        ));
    }
}

/// Spawn debris dust when an asteroid is fully destroyed or scattered.
///
/// `n` controls the density: more fragments = more particles.
pub fn spawn_debris_particles(commands: &mut Commands, pos: Vec2, asteroid_vel: Vec2, n: u32) {
    let mut rng = rand::thread_rng();
    let count = (6 + n * 2).min(16);

    for _ in 0..count {
        let angle = rng.gen_range(0.0_f32..std::f32::consts::TAU);
        let speed = rng.gen_range(30.0_f32..100.0_f32);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed + asteroid_vel * 0.4;

        // Rocky warm-grey dust.
        let lum = rng.gen_range(0.60_f32..0.90_f32);
        let warm = rng.gen_range(0.0_f32..0.12_f32);
        let r = (lum + warm).min(1.0);
        let g = lum;
        let b = (lum - warm * 0.5).max(0.0);

        let lifetime = rng.gen_range(0.25_f32..0.55_f32);
        let offset = Vec2::new(rng.gen_range(-4.0..4.0), rng.gen_range(-4.0..4.0));

        commands.spawn((
            Particle {
                velocity,
                age: 0.0,
                lifetime,
                r,
                g,
                b,
                material: None,
            },
            Transform::from_translation((pos + offset).extend(0.9)),
            Visibility::default(),
        ));
    }
}

/// Spawn a cyan glow burst at `center` when two or more asteroids merge.
pub fn spawn_merge_particles(commands: &mut Commands, center: Vec2) {
    let mut rng = rand::thread_rng();
    let count = 10_u32;

    for _ in 0..count {
        let angle = rng.gen_range(0.0_f32..std::f32::consts::TAU);
        let speed = rng.gen_range(25.0_f32..80.0_f32);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;

        // Cyan-white merge glow.
        let r = rng.gen_range(0.20_f32..0.55_f32);
        let g = rng.gen_range(0.80_f32..1.0_f32);
        let b = rng.gen_range(0.80_f32..1.0_f32);

        let lifetime = rng.gen_range(0.35_f32..0.60_f32);
        let offset = Vec2::new(rng.gen_range(-6.0..6.0), rng.gen_range(-6.0..6.0));

        commands.spawn((
            Particle {
                velocity,
                age: 0.0,
                lifetime,
                r,
                g,
                b,
                material: None,
            },
            Transform::from_translation((center + offset).extend(0.9)),
            Visibility::default(),
        ));
    }
}

/// Spawn light-blue directional particles for tractor beam force application.
///
/// `force_dir` should point in the same direction as the applied tractor force.
/// `intensity` is expected in `[0, 1]` and controls emission count/energy.
pub fn spawn_tractor_beam_particles(
    commands: &mut Commands,
    origin: Vec2,
    force_dir: Vec2,
    target_vel: Vec2,
    mode: TractorBeamVfxMode,
    intensity: f32,
) {
    let mut rng = rand::thread_rng();
    let base_dir = if force_dir.length_squared() > 1e-6 {
        force_dir.normalize()
    } else {
        Vec2::Y
    };
    let base_angle = base_dir.y.atan2(base_dir.x);
    let intensity = intensity.clamp(0.0, 1.0);

    let base_count = match mode {
        TractorBeamVfxMode::Pull => 2_u32,
        TractorBeamVfxMode::Push => 2_u32,
        TractorBeamVfxMode::Freeze => 3_u32,
    };
    let count = (base_count as f32 + intensity * 2.0).round() as u32;

    for _ in 0..count.max(1) {
        let spread = match mode {
            TractorBeamVfxMode::Pull => 0.28_f32,
            TractorBeamVfxMode::Push => 0.22_f32,
            TractorBeamVfxMode::Freeze => 0.45_f32,
        };
        let angle = base_angle + rng.gen_range(-spread..spread);

        let speed = match mode {
            TractorBeamVfxMode::Pull => rng.gen_range(35.0_f32..80.0_f32),
            TractorBeamVfxMode::Push => rng.gen_range(55.0_f32..105.0_f32),
            TractorBeamVfxMode::Freeze => rng.gen_range(20.0_f32..55.0_f32),
        } * (0.7 + 0.6 * intensity);

        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed + target_vel * 0.12;

        let (r, g, b, lifetime) = match mode {
            TractorBeamVfxMode::Pull => (
                rng.gen_range(0.50_f32..0.72_f32),
                rng.gen_range(0.88_f32..1.00_f32),
                rng.gen_range(0.95_f32..1.00_f32),
                rng.gen_range(0.12_f32..0.23_f32),
            ),
            TractorBeamVfxMode::Push => (
                rng.gen_range(0.38_f32..0.62_f32),
                rng.gen_range(0.78_f32..0.95_f32),
                rng.gen_range(0.95_f32..1.00_f32),
                rng.gen_range(0.10_f32..0.20_f32),
            ),
            TractorBeamVfxMode::Freeze => (
                rng.gen_range(0.68_f32..0.90_f32),
                rng.gen_range(0.94_f32..1.00_f32),
                rng.gen_range(0.94_f32..1.00_f32),
                rng.gen_range(0.18_f32..0.32_f32),
            ),
        };

        let lateral = Vec2::new(-base_dir.y, base_dir.x) * rng.gen_range(-2.5_f32..2.5_f32);
        let offset = base_dir * rng.gen_range(-2.0_f32..2.0_f32) + lateral;

        commands.spawn((
            Particle {
                velocity,
                age: 0.0,
                lifetime,
                r,
                g,
                b,
                material: None,
            },
            Transform::from_translation((origin + offset).extend(0.9)),
            Visibility::default(),
        ));
    }
}

/// Spawn light-blue ion particles used by ion shots and stunned enemies.
///
/// `dir_hint` biases the spray direction when non-zero.
pub fn spawn_ion_particles(commands: &mut Commands, origin: Vec2, dir_hint: Vec2, base_vel: Vec2) {
    let mut rng = rand::thread_rng();
    let count = 2_u32;

    let use_dir = if dir_hint.length_squared() > 1e-6 {
        Some(dir_hint.normalize())
    } else {
        None
    };

    for _ in 0..count {
        let velocity = if let Some(dir) = use_dir {
            let angle = dir.y.atan2(dir.x) + rng.gen_range(-0.28_f32..0.28_f32);
            let speed = rng.gen_range(38.0_f32..95.0_f32);
            Vec2::new(angle.cos(), angle.sin()) * speed + base_vel * 0.12
        } else {
            let angle = rng.gen_range(0.0_f32..std::f32::consts::TAU);
            let speed = rng.gen_range(22.0_f32..60.0_f32);
            Vec2::new(angle.cos(), angle.sin()) * speed + base_vel * 0.15
        };

        let r = rng.gen_range(0.45_f32..0.72_f32);
        let g = rng.gen_range(0.88_f32..1.00_f32);
        let b = rng.gen_range(0.95_f32..1.00_f32);
        let lifetime = rng.gen_range(0.10_f32..0.22_f32);

        let lateral = if let Some(dir) = use_dir {
            Vec2::new(-dir.y, dir.x) * rng.gen_range(-1.8_f32..1.8_f32)
        } else {
            Vec2::new(
                rng.gen_range(-1.8_f32..1.8_f32),
                rng.gen_range(-1.8_f32..1.8_f32),
            )
        };

        commands.spawn((
            Particle {
                velocity,
                age: 0.0,
                lifetime,
                r,
                g,
                b,
                material: None,
            },
            Transform::from_translation((origin + lateral).extend(0.9)),
            Visibility::default(),
        ));
    }
}

// ── Mesh helper ───────────────────────────────────────────────────────────────

/// Build a filled circle mesh approximated by an `n`-sided regular polygon.
///
/// Uses a triangle fan from the centre: `(0, i, i+1 mod n)`.
fn circle_mesh(radius: f32, sides: u32) -> Mesh {
    let n = sides as usize;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n + 1);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n + 1);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(n + 1);

    // Centre vertex.
    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 0.0, 1.0]);
    uvs.push([0.5, 0.5]);

    for i in 0..n {
        let angle = std::f32::consts::TAU * i as f32 / n as f32;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        positions.push([x, y, 0.0]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([x / (2.0 * radius) + 0.5, y / (2.0 * radius) + 0.5]);
    }

    let mut indices: Vec<u32> = Vec::with_capacity(n * 3);
    for i in 0..n as u32 {
        // v1 = rim vertex i+1, v2 = next rim vertex wrapping back to 1
        let v1 = i + 1;
        let v2 = (i + 1) % n as u32 + 1;
        indices.extend_from_slice(&[0, v1, v2]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
