# Bevy 0.13 → 0.15+ Migration Plan

## Executive Summary

Upgrade from Bevy 0.13 / Rapier2D 0.18 to Bevy 0.15+ / Rapier2D 0.21+ to access latest features, performance improvements, and security updates. Estimated effort: 4–6 hours across 3 phases.

## Current vs. Target State

| Component | Current | Target | Notes |
| --- | --- | --- | --- |
| Bevy | 0.13 | 0.15.x | Latest stable as of Feb 2026 |
| bevy_rapier2d | 0.25 | 0.27.x | Follows Bevy version lock |
| rapier2d | 0.18 | 0.21.x | Physics engine independently versioned |
| Rust Edition | 2021 | 2021 | No change |

## Phase 1: Dependency Update & Compilation Fixes (1–2 hrs)

### 1.1 Update `Cargo.toml`

```toml
[dependencies]
bevy = { version = "0.15", features = ["dynamic_linking"] }
bevy_rapier2d = { version = "0.27", features = ["simd-stable"] }
rand = "0.8"
rapier2d = { version = "0.21", features = ["simd-stable"] }
```

### 1.2 Key Breaking Changes to Handle

#### **Breaking Change A: `TransformBundle` Removed**

**Old Code** (Bevy 0.13):
```rust
commands.spawn((
    TransformBundle::from_transform(Transform::from_translation(...)),
    VisibilityBundle::default(),
    // ... other components
));
```

**New Code** (Bevy 0.15+):
```rust
commands.spawn((
    Transform::from_translation(...),
    GlobalTransform::default(),
    Visibility::default(),
    // ... other components
));
```

**Files to Update**:
- `src/main.rs` — player spawn
- `src/asteroid.rs` — asteroid spawn
- `src/player/combat.rs` — projectile spawn
- `src/player/mod.rs` — player ship spawn
- `src/simulation.rs` — camera spawn

**Action**: Grep for `TransformBundle::` and replace with explicit Transform + GlobalTransform + Visibility.

---

#### **Breaking Change B: `Text2dBundle` and `Text` API Overhaul**

Bevy 0.14+ completely rewrote text rendering. The `TextBundle` and `Text2dBundle` structures changed significantly.

**Old Code** (Bevy 0.13):
```rust
commands.spawn(Text2dBundle {
    text: Text::from_section(
        "Live: 42",
        TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 20.0,
            color: Color::rgb(0.0, 1.0, 1.0),
        },
    ),
    text_anchor: Anchor::TopLeft,
    transform: Transform::from_translation(Vec3::new(-960.0, 540.0, 100.0)),
    ..default()
});
```

**New Code** (Bevy 0.15+):
```rust
commands.spawn(Text2d::new("Live: 42"))
    .insert(TextFont {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        font_size: 20.0,
        ..default()
    })
    .insert(TextColor(Color::srgb(0.0, 1.0, 1.0)))
    .insert(Anchor::TopLeft)
    .insert(Transform::from_translation(Vec3::new(-960.0, 540.0, 100.0)));
```

**Files to Update**:
- `src/rendering.rs` — `stats_display_system` (if it exists)
- `src/simulation.rs` or similar — any text rendering systems

**Action**: Refactor all text spawning to use the new `Text2d` + `TextFont` + `TextColor` component pattern.

---

#### **Breaking Change C: System Ordering with `apply_deferred`**

Bevy 0.14+ changed how system buffer application works. `.chain()` still works but the semantics may differ.

**Current Code** (Bevy 0.13):
```rust
.add_systems(
    PostUpdate,
    (
        asteroid_formation_system,
        apply_system_buffers.after(asteroid_formation_system),
        projectile_asteroid_hit_system.after(apply_system_buffers),
    ).chain(),
)
```

**New Code** (Bevy 0.15+):
```rust
.add_systems(
    PostUpdate,
    (
        asteroid_formation_system,
        apply_deferred,
        projectile_asteroid_hit_system,
    ).chain(),
)
```

Or use `.after()` directly without explicit `apply_deferred`:

```rust
.add_systems(
    PostUpdate,
    (
        asteroid_formation_system,
        projectile_asteroid_hit_system.after(asteroid_formation_system),
    ),
)
```

**Files to Update**:
- `src/simulation.rs` — system plugin setup (`.add_systems()`)

**Action**: Replace any `apply_system_buffers` calls with `apply_deferred` or restructure to let Bevy handle it automatically via `.after()`.

---

## Phase 2: Code Refactoring (2–3 hrs)

### 2.1 Search & Replace Tasks

Create these search patterns in your editor to find all instances systematically:

| Pattern | Replacement Context | File Pattern |
| --- | --- | --- |
| `TransformBundle::from_transform` | Extract Transform, add GlobalTransform + Visibility | `src/**/*.rs` |
| `Text2dBundle {` | Rewrite to Text2d + components | `src/**/*.rs` |
| `TextStyle {` | Rewrite to TextFont + TextColor | `src/**/*.rs` |
| `apply_system_buffers` | Replace with `apply_deferred` | `src/**/*.rs` |
| `Color::rgb(` | Change to `Color::srgb(` (linear vs sRGB) | `src/**/*.rs` |

### 2.2 Visibility Component Handling

**Old** (Bevy 0.13): `VisibilityBundle { visibility: Visibility::Visible, ..default() }`

**New** (Bevy 0.15+): `Visibility::Visible` directly (or omit for default visible)

All spawns that currently use `VisibilityBundle::default()` can simply add `Visibility::default()` as a separate component.

### 2.3 Color API Change (Linear → sRGB)

Bevy 0.14+ changed color representation. All `Color::rgb(...)` calls should become `Color::srgb(...)` unless you explicitly want linear color space.

**Impact**: Roughly 15–20 instances across the codebase (player color, asteroid colors, aim indicator, etc.).

---

## Phase 3: Testing & Validation (1–2 hrs)

### 3.1 Compilation Check
```bash
cargo check 2>&1
```

Expected: zero errors. If you see compilation errors, they will be specific to the remaining API changes.

### 3.2 Clippy Lint
```bash
cargo clippy -- -D warnings 2>&1
```

Fix any new clippy warnings introduced by the migration.

### 3.3 Build & Run
```bash
cargo build --release
cargo run --release
```

Verify the simulation launches without panics.

### 3.4 Physics Tests
```bash
./test_all.sh
```

Run all automated tests to ensure physics behavior is unchanged. All 11 tests should still pass.

### 3.5 Manual Smoke Test
- Verify the window opens and game initializes
- Verify asteroids spawn on click
- Verify projectiles fire and collide with asteroids
- Verify camera follows and zooms
- Verify aim indicator displays

---

## Detailed File Changes by Module

### `src/main.rs` (Player Spawn)

**Change**: Replace `TransformBundle::from_transform()` with explicit components.

```rust
// OLD (0.13)
commands.spawn(/* ... other components ..., TransformBundle::from_transform(...), ... */);

// NEW (0.15+)
commands.spawn((
    /* ... other components ... */
    Transform::from_translation(...),
    GlobalTransform::default(),
    Visibility::default(),
));
```

### `src/asteroid.rs` (Asteroid Spawning)

Same pattern: Replace `TransformBundle::from_transform()` + `VisibilityBundle::default()` with:
- `Transform::from_translation(...)`
- `GlobalTransform::default()`
- `Visibility::default()` (or omit as it's the default)

Also update `Color::rgb(...)` → `Color::srgb(...)` for randomly-generated grey values.

### `src/player/combat.rs` (Projectile Spawn)

Replace `TransformBundle::from_transform()` + `VisibilityBundle::default()` in projectile spawn.

```rust
commands.spawn((
    Projectile { age: 0.0 },
    Transform::from_translation(spawn_pos.extend(0.0)),
    GlobalTransform::default(),
    Visibility::default(),
    // ... rest of projectile components
));
```

### `src/simulation.rs` (Text, Camera, Systems)

#### Text Display
If you have stats text rendering:

```rust
// OLD (0.13)
commands.spawn(Text2dBundle { 
    text: Text::from_section(...), 
    text_anchor: Anchor::TopLeft, 
    transform: Transform::from_translation(...), 
    .. 
});

// NEW (0.15+)
commands.spawn((
    Text2d::new("Live: 42"),
    TextFont {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        font_size: 20.0,
        ..default()
    },
    TextColor(Color::srgb(0.0, 1.0, 1.0)),
    Anchor::TopLeft,
    Transform::from_translation(Vec3::new(...)),
    GlobalTransform::default(),
    Visibility::default(),
));
```

#### Camera Spawn
Replace `TransformBundle::from_transform()` with explicit components.

#### System Ordering
If you use `apply_system_buffers` explicitly, replace with `apply_deferred`.

---

## Compatibility Notes

### Physics Behavior

Rapier2D 0.21 is largely API-compatible with 0.18 at the component level. No breaking changes expected in:
- `RigidBody`, `Velocity`, `Collider`, `CollisionGroups`, `ActiveEvents`, `Ccd`, etc.
- Contact detection and response logic is unchanged

### Performance

Bevy 0.15+ generally has better performance than 0.13:
- Improved scheduling overhead
- Better memory layout in ECS
- Faster text rendering pipeline
- Potential FPS improvement especially at high asteroid counts

### Features You Gain

1. **Bevy UI improvements** — if you ever add UI (not currently used)
2. **Better diagnostic tools** — for profiling and debugging
3. **Improved asset handling** — more robust loading/unloading
4. **Latest security patches** — dependency vulnerabilities fixed

---

## Rollback Plan

If the migration encounters unexpected issues:

1. Create a git branch: `git checkout -b bevy-0.15-migration`
2. If migration fails beyond recovery, revert: `git checkout main`
3. Keep the branch for future retry once Bevy APIs stabilize further

---

## Effort Estimation

| Phase | Task | Duration | Difficulty |
| --- | --- | --- | --- |
| 1.1 | Update Cargo.toml | 5 min | Trivial |
| 1.2 | First `cargo check` | 2–5 min | Low (compile errors will guide fixes) |
| 2 | Code refactoring | 2–3 hrs | Medium (repetitive but straightforward) |
| 3 | Testing & validation | 1–2 hrs | Low (automated tests verify correctness) |
| **Total** | **Full migration** | **4–6 hrs** | **Medium** |

---

## Migration Checklist

- [ ] Create git branch: `git checkout -b bevy-0.15-migration`
- [ ] Update `Cargo.toml` with new versions
- [ ] Run `cargo check` and note all compilation errors
- [ ] Replace all `TransformBundle::from_transform()` calls (Phase 2.1)
- [ ] Replace all `VisibilityBundle::default()` with `Visibility::default()`
- [ ] Refactor text rendering (Text2dBundle → Text2d pattern)
- [ ] Replace `apply_system_buffers` with `apply_deferred`
- [ ] Replace `Color::rgb(...)` with `Color::srgb(...)`
- [ ] Run `cargo clippy -- -D warnings` (fix any new warnings)
- [ ] Run `cargo build --release` (full compilation)
- [ ] Run `cargo run --release` (manual smoke test)
- [ ] Run `./test_all.sh` (automated test suite)
- [ ] Verify all 11 physics tests pass
- [ ] Merge: `git checkout main && git merge bevy-0.15-migration`
- [ ] Commit: `git commit -m "Migrate to Bevy 0.15 + Rapier2D 0.21"`

---

## Post-Migration Opportunities

Once migration is complete, consider:

1. **Upgrade CHANGELOG** — add "Upgraded to Bevy 0.15" section
2. **Update copilot-instructions.md** — reference new Bevy features
3. **Explore new Bevy 0.15 features** — animation system, improved diagnostics, etc.
4. **Performance profiling** — use new diagnostic tools to optimize further
5. **Plan for Bevy 0.16** — check roadmap for next migration window

---

## References

- **Bevy 0.14 Migration Guide**: https://bevyengine.org/learn/book/migration-guides/0.13-0.14/
- **Bevy 0.15 Migration Guide**: https://bevyengine.org/learn/book/migration-guides/0.14-0.15/
- **bevy_rapier2d 0.27 Docs**: https://docs.rs/bevy_rapier2d/0.27/bevy_rapier2d/
- **Bevy Text Rendering**: https://bevyengine.org/learn/book/getting_started/text/

