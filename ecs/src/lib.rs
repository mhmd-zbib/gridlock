use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Entity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Entity(u64);

impl Entity {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn id(self) -> u64 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// Marker traits
// ---------------------------------------------------------------------------

/// Marker for data that can be stored as an entity component.
///
/// The game tick loop is single-threaded so Send + Sync are not required.
pub trait Component: 'static {}

/// Marker for event types dispatched through the EventBus.
pub trait Event: 'static {}

/// Marker for singleton global resources stored in the World.
pub trait Resource: 'static {}

// ---------------------------------------------------------------------------
// System trait
// ---------------------------------------------------------------------------

pub trait System {
    fn update(&mut self, world: &mut World, dt: f32);
}

impl<F: FnMut(&mut World, f32)> System for F {
    fn update(&mut self, world: &mut World, dt: f32) {
        (self)(world, dt);
    }
}

// ---------------------------------------------------------------------------
// Type-erased component storage
// ---------------------------------------------------------------------------

trait ErasedComponentStore {
    fn remove_entity(&mut self, entity: Entity);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct ComponentStore<C: Component> {
    data: HashMap<Entity, C>,
}

impl<C: Component> ErasedComponentStore for ComponentStore<C> {
    fn remove_entity(&mut self, entity: Entity) {
        self.data.remove(&entity);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Type-erased event queue
// ---------------------------------------------------------------------------

trait ErasedEventQueue {
    fn clear(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct EventQueue<E: Event> {
    events: Vec<E>,
}

impl<E: Event> ErasedEventQueue for EventQueue<E> {
    fn clear(&mut self) {
        self.events.clear();
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// EventBus
// ---------------------------------------------------------------------------

pub struct EventBus {
    queues: HashMap<TypeId, Box<dyn ErasedEventQueue>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self { queues: HashMap::new() }
    }

    fn queue_mut<E: Event>(&mut self) -> &mut EventQueue<E> {
        self.queues
            .entry(TypeId::of::<E>())
            .or_insert_with(|| Box::new(EventQueue::<E> { events: Vec::new() }))
            .as_any_mut()
            .downcast_mut::<EventQueue<E>>()
            .unwrap()
    }

    fn queue<E: Event>(&self) -> Option<&EventQueue<E>> {
        self.queues
            .get(&TypeId::of::<E>())?
            .as_any()
            .downcast_ref::<EventQueue<E>>()
    }

    pub fn emit<E: Event>(&mut self, event: E) {
        self.queue_mut::<E>().events.push(event);
    }

    pub fn drain<E: Event>(&mut self) -> Vec<E> {
        std::mem::take(&mut self.queue_mut::<E>().events)
    }

    pub fn read<E: Event>(&self) -> &[E] {
        self.queue::<E>().map(|q| q.events.as_slice()).unwrap_or(&[])
    }

    pub fn clear_all(&mut self) {
        for q in self.queues.values_mut() {
            q.clear();
        }
    }
}

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

pub struct World {
    next_id: u64,
    alive: HashSet<Entity>,
    components: HashMap<TypeId, Box<dyn ErasedComponentStore>>,
    resources: HashMap<TypeId, Box<dyn Any>>,
    pub events: EventBus,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            alive: HashSet::new(),
            components: HashMap::new(),
            resources: HashMap::new(),
            events: EventBus::new(),
        }
    }

    // --- Entity management ---

    pub fn spawn(&mut self) -> Entity {
        let entity = Entity::new(self.next_id);
        self.next_id += 1;
        self.alive.insert(entity);
        entity
    }

    pub fn despawn(&mut self, entity: Entity) {
        if self.alive.remove(&entity) {
            for store in self.components.values_mut() {
                store.remove_entity(entity);
            }
        }
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.alive.contains(&entity)
    }

    pub fn all_entities(&self) -> Vec<Entity> {
        self.alive.iter().copied().collect()
    }

    // --- Component management ---

    fn store<C: Component>(&self) -> Option<&ComponentStore<C>> {
        self.components
            .get(&TypeId::of::<C>())?
            .as_any()
            .downcast_ref::<ComponentStore<C>>()
    }

    fn store_or_create<C: Component>(&mut self) -> &mut ComponentStore<C> {
        self.components
            .entry(TypeId::of::<C>())
            .or_insert_with(|| {
                Box::new(ComponentStore::<C> { data: HashMap::new() })
            })
            .as_any_mut()
            .downcast_mut::<ComponentStore<C>>()
            .unwrap()
    }

    pub fn insert<C: Component>(&mut self, entity: Entity, component: C) {
        self.store_or_create::<C>().data.insert(entity, component);
    }

    pub fn remove<C: Component>(&mut self, entity: Entity) -> Option<C> {
        self.components
            .get_mut(&TypeId::of::<C>())?
            .as_any_mut()
            .downcast_mut::<ComponentStore<C>>()?
            .data
            .remove(&entity)
    }

    pub fn get<C: Component>(&self, entity: Entity) -> Option<&C> {
        self.store::<C>()?.data.get(&entity)
    }

    pub fn get_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
        self.store_or_create::<C>().data.get_mut(&entity)
    }

    pub fn has<C: Component>(&self, entity: Entity) -> bool {
        self.store::<C>().is_some_and(|s| s.data.contains_key(&entity))
    }

    /// All entities that have component C.
    pub fn entities_with<C: Component>(&self) -> Vec<Entity> {
        self.store::<C>()
            .map(|s| s.data.keys().copied().collect())
            .unwrap_or_default()
    }

    /// Iterate all (entity, &C) pairs.
    pub fn iter<C: Component>(&self) -> impl Iterator<Item = (Entity, &C)> {
        self.store::<C>()
            .into_iter()
            .flat_map(|s| s.data.iter().map(|(&e, c)| (e, c)))
    }

    /// Call `f` for every (entity, &mut C) pair.
    pub fn for_each_mut<C: Component, F: FnMut(Entity, &mut C)>(&mut self, mut f: F) {
        if let Some(store) = self.components
            .get_mut(&TypeId::of::<C>())
            .and_then(|s| s.as_any_mut().downcast_mut::<ComponentStore<C>>())
        {
            for (&entity, component) in store.data.iter_mut() {
                f(entity, component);
            }
        }
    }

    // --- Resource management ---

    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.resources.insert(TypeId::of::<R>(), Box::new(resource) as Box<dyn Any>);
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.resources.get(&TypeId::of::<R>())?.downcast_ref::<R>()
    }

    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.resources.get_mut(&TypeId::of::<R>())?.downcast_mut::<R>()
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.get_resource::<R>().expect("resource not found")
    }

    pub fn resource_mut<R: Resource>(&mut self) -> &mut R {
        self.get_resource_mut::<R>().expect("resource not found")
    }

    pub fn contains_resource<R: Resource>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<R>())
    }
}
