//! [`DeterministicSim`] — the step-based deterministic simulation engine.

use std::collections::BTreeMap;

/// A unique identifier for an entity within a [`DeterministicSim`].
///
/// IDs are assigned monotonically from zero and are never reused within a
/// single simulation run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(u64);

type SystemFn<S> = Box<dyn Fn(&mut BTreeMap<EntityId, S>)>;

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
/// - **Arithmetic**: controlled by the caller's choice of `S`. Use
///   [`FixedPoint`](crate::FixedPoint) or the `exact-math` feature for
///   guaranteed cross-platform determinism.
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
/// sim.add_system("increment", |entities| {
///     for v in entities.values_mut() {
///         *v = *v + FixedPoint::from_int(1);
///     }
/// });
///
/// sim.step();
/// assert_eq!(*sim.get(a).unwrap(), Fp::from_int(2));
/// assert_eq!(*sim.get(b).unwrap(), Fp::from_int(3));
/// ```
pub struct DeterministicSim<S: Clone + std::fmt::Debug> {
    entities: BTreeMap<EntityId, S>,
    systems: BTreeMap<String, SystemFn<S>>,
    next_id: u64,
    step_count: u64,
}

impl<S: Clone + std::fmt::Debug> DeterministicSim<S> {
    /// Creates a new, empty simulation.
    pub fn new() -> Self {
        Self {
            entities: BTreeMap::new(),
            systems: BTreeMap::new(),
            next_id: 0,
            step_count: 0,
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

    /// Returns how many steps have been executed since creation.
    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    /// Registers a system function under a unique name.
    ///
    /// Systems are executed in alphabetical order by name on every
    /// [`step`](Self::step) call. If a system with the same name already
    /// exists it is replaced.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_deterministic_sim::{DeterministicSim, FixedPoint};
    /// type Fp = FixedPoint<1_000>;
    /// let mut sim = DeterministicSim::<Fp>::new();
    /// sim.add_system("gravity", |_entities| { /* apply gravity */ });
    /// ```
    pub fn add_system(
        &mut self,
        name: impl Into<String>,
        system: impl Fn(&mut BTreeMap<EntityId, S>) + 'static,
    ) {
        self.systems.insert(name.into(), Box::new(system));
    }

    /// Removes a previously registered system by name.
    ///
    /// Returns `true` if the system existed and was removed.
    pub fn remove_system(&mut self, name: &str) -> bool {
        self.systems.remove(name).is_some()
    }

    /// Advances the simulation by one step, running all systems in name order.
    ///
    /// This is the core simulation loop. For determinism, every call with the
    /// same initial state must produce the same output state.
    pub fn step(&mut self) {
        // Systems are stored in a BTreeMap so iteration is always alphabetically sorted.
        // We must collect keys first to avoid borrowing conflicts.
        let names: Vec<String> = self.systems.keys().cloned().collect();
        for name in &names {
            if let Some(system) = self.systems.get(name) {
                system(&mut self.entities);
            }
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
}

impl<S: Clone + std::fmt::Debug> Default for DeterministicSim<S> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FixedPoint;

    type Fp = FixedPoint<1_000>;

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
        sim.add_system("double", |entities| {
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
        sim.add_system("b_second", |entities| {
            for v in entities.values_mut() {
                *v = *v + Fp::from_int(1);
            }
        });
        sim.add_system("a_first", |entities| {
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
            sim.add_system("scale", |entities| {
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
}
