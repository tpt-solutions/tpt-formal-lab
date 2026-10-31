//! [`DeterministicSim`] — the step-based deterministic simulation engine.

use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};

/// A unique identifier for an entity within a [`DeterministicSim`].
///
/// IDs are assigned monotonically from zero and are never reused within a
/// single simulation run, including IDs reserved by [`SimContext::spawn`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(u64);

/// The entity map that systems read and write.
pub type Entities<S> = BTreeMap<EntityId, S>;

type SystemFn<S> = Box<dyn FnMut(&mut Entities<S>, &mut SimContext<S>)>;

/// Per-step state and deferred structural changes, passed to every system.
///
/// Spawns and despawns are queued and applied after all systems have run, so
/// the entity map is not changed while a system is iterating it. Spawns are
/// applied first, then despawns, both in the order they were queued.
pub struct SimContext<S> {
    step: u64,
    timestep: Option<S>,
    next_id: u64,
    spawns: Vec<(EntityId, S)>,
    despawns: Vec<EntityId>,
}

impl<S> SimContext<S> {
    /// Returns the index of the step being run, starting at zero.
    pub fn step(&self) -> u64 {
        self.step
    }

    /// Returns the timestep set with [`DeterministicSim::with_timestep`], if any.
    pub fn timestep(&self) -> Option<&S> {
        self.timestep.as_ref()
    }

    /// Queues a new entity and returns its ID straight away, so later systems
    /// in the same step can refer to it.
    pub fn spawn(&mut self, state: S) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.spawns.push((id, state));
        id
    }

    /// Queues the removal of an entity. Removing an ID that does not exist is
    /// a no-op.
    pub fn despawn(&mut self, id: EntityId) {
        self.despawns.push(id);
    }
}

/// A saved copy of a simulation's entities and counters. Restore it with
/// [`DeterministicSim::restore`]. Systems are not part of a snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot<S> {
    entities: Entities<S>,
    next_id: u64,
    step_count: u64,
}

/// A step-based, bitwise-deterministic simulation.
///
/// `DeterministicSim<S>` stores a collection of entities, each with a state
/// value of type `S`. On each call to [`step`](Self::step), all registered
/// systems are applied to the entity map in **alphabetical name order**,
/// ensuring identical execution order on every run.
///
/// # Determinism guarantees
///
/// - **Entity storage**: `BTreeMap` — iteration order is always sorted by
///   [`EntityId`], never random.
/// - **System order**: systems are sorted by name string at registration time.
/// - **Structural changes**: spawns and despawns requested by systems are applied
///   after all systems run, in the order they were requested.
/// - **Arithmetic**: controlled by the caller's choice of `S`. Use
///   [`FixedPoint`](crate::FixedPoint) for guaranteed cross-platform
///   determinism.
/// - **State hash**: [`state_hash`](Self::state_hash) writes integers in
///   little-endian order, so it gives the same value on every CPU.
///
/// # Example
///
/// ```rust
/// use tpt_deterministic_sim::{DeterministicSim, FixedPoint};
///
/// type Fp = FixedPoint<1_000>;
/// let mut sim = DeterministicSim::<Fp>::new();
///
/// let a = sim.spawn(Fp::from_int(1));
/// let b = sim.spawn(Fp::from_int(2));
///
/// sim.add_system("increment", |entities, _ctx| {
///     for v in entities.values_mut() {
///         *v = *v + FixedPoint::from_int(1);
///     }
/// });
///
/// sim.step();
/// assert_eq!(*sim.get(a).unwrap(), Fp::from_int(2));
/// assert_eq!(*sim.get(b).unwrap(), Fp::from_int(3));
/// ```
pub struct DeterministicSim<S: Clone + fmt::Debug> {
    entities: Entities<S>,
    systems: BTreeMap<String, SystemFn<S>>,
    timestep: Option<S>,
    next_id: u64,
    step_count: u64,
}

impl<S: Clone + fmt::Debug> DeterministicSim<S> {
    /// Creates a new, empty simulation.
    pub fn new() -> Self {
        Self {
            entities: BTreeMap::new(),
            systems: BTreeMap::new(),
            timestep: None,
            next_id: 0,
            step_count: 0,
        }
    }

    /// Creates a new, empty simulation with a timestep that systems can read
    /// through [`SimContext::timestep`].
    pub fn with_timestep(dt: S) -> Self {
        Self {
            timestep: Some(dt),
            ..Self::new()
        }
    }

    /// Spawns a new entity with the given initial state, returning its [`EntityId`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_deterministic_sim::{DeterministicSim, FixedPoint};
    /// type Fp = FixedPoint<1_000>;
    /// let mut sim = DeterministicSim::<Fp>::new();
    /// let id = sim.spawn(Fp::from_int(42));
    /// assert_eq!(*sim.get(id).unwrap(), Fp::from_int(42));
    /// ```
    pub fn spawn(&mut self, state: S) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.entities.insert(id, state);
        id
    }

    /// Removes an entity from the simulation.
    ///
    /// Returns the entity's final state, or `None` if the id was not found.
    pub fn despawn(&mut self, id: EntityId) -> Option<S> {
        self.entities.remove(&id)
    }

    /// Returns a reference to an entity's state, or `None` if not found.
    pub fn get(&self, id: EntityId) -> Option<&S> {
        self.entities.get(&id)
    }

    /// Returns a mutable reference to an entity's state, or `None` if not found.
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut S> {
        self.entities.get_mut(&id)
    }

    /// Returns the number of active entities.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Returns how many steps have been executed since creation (or since the
    /// snapshot that was last restored).
    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    /// Registers a system function under a unique name.
    ///
    /// Systems run in alphabetical order by name on every [`step`](Self::step)
    /// call. A system may keep state between steps, because it is `FnMut`. It
    /// receives the entity map and a [`SimContext`]. If a system with the same
    /// name already exists it is replaced.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_deterministic_sim::{DeterministicSim, FixedPoint};
    /// type Fp = FixedPoint<1_000>;
    /// let mut sim = DeterministicSim::<Fp>::new();
    /// sim.add_system("gravity", |_entities, _ctx| { /* apply gravity */ });
    /// ```
    pub fn add_system(
        &mut self,
        name: impl Into<String>,
        system: impl FnMut(&mut Entities<S>, &mut SimContext<S>) + 'static,
    ) {
        self.systems.insert(name.into(), Box::new(system));
    }

    /// Removes a previously registered system by name.
    ///
    /// Returns `true` if the system existed and was removed.
    pub fn remove_system(&mut self, name: &str) -> bool {
        self.systems.remove(name).is_some()
    }

    /// Advances the simulation by one step, running all systems in name order,
    /// then applies the spawns and despawns they queued.
    ///
    /// This is the core simulation loop. For determinism, every call with the
    /// same initial state must produce the same output state.
    pub fn step(&mut self) {
        let mut ctx = SimContext {
            step: self.step_count,
            timestep: self.timestep.clone(),
            next_id: self.next_id,
            spawns: Vec::new(),
            despawns: Vec::new(),
        };
        // `BTreeMap` iteration is already sorted by name.
        for system in self.systems.values_mut() {
            system(&mut self.entities, &mut ctx);
        }
        self.next_id = ctx.next_id;
        for (id, state) in ctx.spawns {
            self.entities.insert(id, state);
        }
        for id in ctx.despawns {
            self.entities.remove(&id);
        }
        self.step_count += 1;
    }

    /// Advances the simulation by `n` steps.
    pub fn step_n(&mut self, n: u64) {
        for _ in 0..n {
            self.step();
        }
    }

    /// Returns an iterator over `(EntityId, &S)` pairs in sorted ID order.
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &S)> {
        self.entities.iter().map(|(&id, s)| (id, s))
    }

    /// Saves the entities and counters. Systems are not saved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_deterministic_sim::{DeterministicSim, FixedPoint};
    /// type Fp = FixedPoint<1_000>;
    /// let mut sim = DeterministicSim::<Fp>::new();
    /// let id = sim.spawn(Fp::from_int(1));
    /// let saved = sim.snapshot();
    /// sim.spawn(Fp::from_int(2));
    /// sim.restore(&saved);
    /// assert_eq!(sim.entity_count(), 1);
    /// assert_eq!(*sim.get(id).unwrap(), Fp::from_int(1));
    /// ```
    pub fn snapshot(&self) -> Snapshot<S> {
        Snapshot {
            entities: self.entities.clone(),
            next_id: self.next_id,
            step_count: self.step_count,
        }
    }

    /// Returns the simulation to a saved state. Systems and the timestep are
    /// kept. IDs that were used after the snapshot are reused by later spawns,
    /// which is the same as replaying from that point.
    pub fn restore(&mut self, snapshot: &Snapshot<S>) {
        self.entities = snapshot.entities.clone();
        self.next_id = snapshot.next_id;
        self.step_count = snapshot.step_count;
    }
}

impl<S: Clone + fmt::Debug + Hash> DeterministicSim<S> {
    /// Returns a hash of the step count and every entity, in ID order.
    ///
    /// The hash uses FNV-1a over little-endian integer encodings, so it is the
    /// same on every CPU architecture and across Rust versions. Two simulations
    /// with equal states give equal hashes. Use it to compare runs, for example
    /// to find the first step where two runs diverge.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_deterministic_sim::{DeterministicSim, FixedPoint};
    /// type Fp = FixedPoint<1_000>;
    /// let mut a = DeterministicSim::<Fp>::new();
    /// let mut b = DeterministicSim::<Fp>::new();
    /// a.spawn(Fp::from_int(1));
    /// b.spawn(Fp::from_int(1));
    /// assert_eq!(a.state_hash(), b.state_hash());
    /// b.spawn(Fp::from_int(2));
    /// assert_ne!(a.state_hash(), b.state_hash());
    /// ```
    pub fn state_hash(&self) -> u64 {
        let mut hasher = StableHasher::new();
        self.step_count.hash(&mut hasher);
        self.entities.len().hash(&mut hasher);
        for (id, state) in &self.entities {
            id.hash(&mut hasher);
            state.hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl<S: Clone + fmt::Debug> Default for DeterministicSim<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// FNV-1a hasher whose integer writes are little-endian, so a hash does not
/// depend on the host's byte order.
struct StableHasher {
    state: u64,
}

impl StableHasher {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;

    fn new() -> Self {
        Self {
            state: Self::OFFSET_BASIS,
        }
    }

    fn absorb(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state ^= u64::from(b);
            self.state = self.state.wrapping_mul(Self::PRIME);
        }
    }
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        self.absorb(bytes);
    }

    fn write_u8(&mut self, i: u8) {
        self.absorb(&i.to_le_bytes());
    }

    fn write_u16(&mut self, i: u16) {
        self.absorb(&i.to_le_bytes());
    }

    fn write_u32(&mut self, i: u32) {
        self.absorb(&i.to_le_bytes());
    }

    fn write_u64(&mut self, i: u64) {
        self.absorb(&i.to_le_bytes());
    }

    fn write_u128(&mut self, i: u128) {
        self.absorb(&i.to_le_bytes());
    }

    fn write_usize(&mut self, i: usize) {
        // Widen to a fixed width so the hash does not depend on pointer size.
        self.absorb(&(i as u64).to_le_bytes());
    }

    fn write_i8(&mut self, i: i8) {
        self.absorb(&i.to_le_bytes());
    }

    fn write_i16(&mut self, i: i16) {
        self.absorb(&i.to_le_bytes());
    }

    fn write_i32(&mut self, i: i32) {
        self.absorb(&i.to_le_bytes());
    }

    fn write_i64(&mut self, i: i64) {
        self.absorb(&i.to_le_bytes());
    }

    fn write_i128(&mut self, i: i128) {
        self.absorb(&i.to_le_bytes());
    }

    fn write_isize(&mut self, i: isize) {
        self.absorb(&(i as i64).to_le_bytes());
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FixedPoint;

    type Fp = FixedPoint<1_000>;

    fn increment(entities: &mut Entities<Fp>, _ctx: &mut SimContext<Fp>) {
        for v in entities.values_mut() {
            *v = *v + Fp::from_int(1);
        }
    }

    #[test]
    fn spawn_and_retrieve() {
        let mut sim = DeterministicSim::<Fp>::new();
        let id = sim.spawn(Fp::from_int(42));
        assert_eq!(*sim.get(id).unwrap(), Fp::from_int(42));
    }

    #[test]
    fn despawn() {
        let mut sim = DeterministicSim::<Fp>::new();
        let id = sim.spawn(Fp::from_int(1));
        assert!(sim.despawn(id).is_some());
        assert!(sim.get(id).is_none());
        assert_eq!(sim.entity_count(), 0);
    }

    #[test]
    fn system_runs_on_step() {
        let mut sim = DeterministicSim::<Fp>::new();
        let id = sim.spawn(Fp::from_int(10));
        sim.add_system("double", |entities, _ctx| {
            for v in entities.values_mut() {
                *v = *v + *v;
            }
        });
        sim.step();
        assert_eq!(*sim.get(id).unwrap(), Fp::from_int(20));
    }

    #[test]
    fn systems_run_in_name_order() {
        // "a_first" runs before "b_second" alphabetically.
        // a_first: set to 10; b_second: add 1 → final = 11
        let mut sim = DeterministicSim::<Fp>::new();
        let id = sim.spawn(Fp::zero());
        sim.add_system("b_second", increment);
        sim.add_system("a_first", |entities, _ctx| {
            for v in entities.values_mut() {
                *v = Fp::from_int(10);
            }
        });
        sim.step();
        assert_eq!(*sim.get(id).unwrap(), Fp::from_int(11));
    }

    #[test]
    fn iteration_order_is_sorted() {
        let mut sim = DeterministicSim::<Fp>::new();
        let id0 = sim.spawn(Fp::from_int(0));
        let id1 = sim.spawn(Fp::from_int(1));
        let id2 = sim.spawn(Fp::from_int(2));
        let ids: Vec<EntityId> = sim.iter().map(|(id, _)| id).collect();
        assert_eq!(ids, vec![id0, id1, id2]);
    }

    #[test]
    fn identical_runs_produce_identical_output() {
        fn run_sim() -> Fp {
            let mut sim = DeterministicSim::<Fp>::new();
            let id = sim.spawn(Fp::from_raw(314));
            sim.add_system("scale", |entities, _ctx| {
                for v in entities.values_mut() {
                    *v = *v + Fp::from_raw(100);
                }
            });
            sim.step_n(5);
            *sim.get(id).unwrap()
        }

        let a = run_sim();
        let b = run_sim();
        assert_eq!(a, b);
        assert_eq!(a.raw(), 814);
    }

    #[test]
    fn step_count_tracked() {
        let mut sim = DeterministicSim::<Fp>::new();
        sim.step_n(7);
        assert_eq!(sim.step_count(), 7);
    }

    #[test]
    fn remove_system_stops_it_running() {
        let mut sim = DeterministicSim::<Fp>::new();
        let id = sim.spawn(Fp::zero());
        sim.add_system("inc", increment);
        sim.step();
        assert!(sim.remove_system("inc"));
        assert!(!sim.remove_system("inc"));
        sim.step_n(3);
        assert_eq!(*sim.get(id).unwrap(), Fp::from_int(1));
    }

    #[test]
    fn despawned_entity_is_skipped_by_systems() {
        let mut sim = DeterministicSim::<Fp>::new();
        let keep = sim.spawn(Fp::from_int(1));
        let gone = sim.spawn(Fp::from_int(1));
        sim.add_system("inc", increment);
        assert_eq!(sim.despawn(gone), Some(Fp::from_int(1)));
        assert_eq!(sim.despawn(gone), None);
        sim.step();
        assert_eq!(*sim.get(keep).unwrap(), Fp::from_int(2));
        assert!(sim.get(gone).is_none());
        assert_eq!(sim.iter().count(), 1);
    }

    #[test]
    fn get_mut_updates_state_between_steps() {
        let mut sim = DeterministicSim::<Fp>::new();
        let id = sim.spawn(Fp::from_int(5));
        *sim.get_mut(id).unwrap() = Fp::from_int(7);
        assert_eq!(*sim.get(id).unwrap(), Fp::from_int(7));
        assert!(sim.get_mut(EntityId(999)).is_none());
    }

    #[test]
    fn ids_are_not_reused_after_despawn() {
        let mut sim = DeterministicSim::<Fp>::new();
        let a = sim.spawn(Fp::zero());
        sim.despawn(a);
        let b = sim.spawn(Fp::zero());
        assert_ne!(a, b);
    }

    #[test]
    fn systems_can_keep_state_between_steps() {
        // An FnMut system counts how often it has run.
        let mut sim = DeterministicSim::<Fp>::new();
        let id = sim.spawn(Fp::zero());
        let mut calls = 0;
        sim.add_system("counter", move |entities, _ctx| {
            calls += 1;
            for v in entities.values_mut() {
                *v = Fp::from_int(calls);
            }
        });
        sim.step_n(3);
        assert_eq!(*sim.get(id).unwrap(), Fp::from_int(3));
    }

    #[test]
    fn context_reports_step_and_timestep() {
        let mut sim = DeterministicSim::<Fp>::with_timestep(Fp::from_raw(250));
        let id = sim.spawn(Fp::zero());
        sim.add_system("record", move |entities, ctx| {
            let dt = *ctx.timestep().unwrap();
            for v in entities.values_mut() {
                *v = *v + Fp::from_int(i64::try_from(ctx.step()).unwrap()) * dt;
            }
        });
        sim.step_n(4);
        // Steps 0..4 add 0, 1, 2, 3 timesteps of 0.25 each: 1.5 in total.
        assert_eq!(*sim.get(id).unwrap(), Fp::from_raw(1_500));
    }

    #[test]
    fn spawn_and_despawn_from_a_system_apply_after_the_systems() {
        let mut sim = DeterministicSim::<Fp>::new();
        let parent = sim.spawn(Fp::from_int(1));
        sim.add_system("spawner", move |entities, ctx| {
            if ctx.step() == 0 {
                let child = ctx.spawn(Fp::from_int(100));
                // The child ID is known straight away, but it is not in the map yet.
                assert!(entities.get(&child).is_none());
            }
            if ctx.step() == 1 {
                ctx.despawn(parent);
            }
        });
        sim.step();
        assert_eq!(sim.entity_count(), 2);
        sim.step();
        assert_eq!(sim.entity_count(), 1);
        assert!(sim.get(parent).is_none());
    }

    #[test]
    fn spawn_ids_from_systems_are_unique() {
        let mut sim = DeterministicSim::<Fp>::new();
        sim.add_system("many", |_entities, ctx| {
            let a = ctx.spawn(Fp::zero());
            let b = ctx.spawn(Fp::zero());
            assert_ne!(a, b);
        });
        sim.step_n(3);
        assert_eq!(sim.entity_count(), 6);
    }

    #[test]
    fn snapshot_and_restore_round_trip() {
        let mut sim = DeterministicSim::<Fp>::new();
        let id = sim.spawn(Fp::from_int(0));
        sim.add_system("inc", increment);
        sim.step_n(2);
        let saved = sim.snapshot();
        let hash_at_save = sim.state_hash();

        sim.step_n(5);
        assert_ne!(sim.state_hash(), hash_at_save);

        sim.restore(&saved);
        assert_eq!(sim.step_count(), 2);
        assert_eq!(*sim.get(id).unwrap(), Fp::from_int(2));
        assert_eq!(sim.state_hash(), hash_at_save);

        // Replaying from the snapshot reproduces the same run.
        sim.step_n(5);
        assert_eq!(*sim.get(id).unwrap(), Fp::from_int(7));
    }

    #[test]
    fn state_hash_is_stable_across_builds() {
        // Golden values: FNV-1a over little-endian encodings. A change here
        // means the hash format changed, which breaks recorded traces.
        let empty = DeterministicSim::<Fp>::new();
        assert_eq!(empty.state_hash(), 0x8820_1fb9_60ff_6465);

        let mut one = DeterministicSim::<Fp>::new();
        one.spawn(Fp::from_int(1));
        assert_eq!(one.state_hash(), 0xd0eb_ac7d_0788_803d);
    }

    #[test]
    fn state_hash_separates_step_counts() {
        let mut a = DeterministicSim::<Fp>::new();
        let b = DeterministicSim::<Fp>::new();
        assert_eq!(a.state_hash(), b.state_hash());
        a.step();
        assert_ne!(a.state_hash(), b.state_hash());
    }
}
