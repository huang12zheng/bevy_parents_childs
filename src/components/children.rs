use bevy_ecs::{component::Component, entity::Entity, prelude::FromWorld, world::World};

use std::{
    collections::BTreeSet,
    ops::{Deref, DerefMut},
};

/// Contains references to the child entities of this entity.
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
#[derive(Component, Debug)]

pub struct Children(pub(crate) BTreeSet<Entity>);

// TODO: We need to impl either FromWorld or Default so Children can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However Children should only ever be set with a real user-defined entities. Its worth looking
// into better ways to handle cases like this.
impl FromWorld for Children {
    fn from_world(_world: &mut World) -> Self {
        Children(BTreeSet::new())
    }
}

impl Children {
    /// Constructs a [`Children`] component with the given entities.
    pub(crate) fn new(entities: BTreeSet<Entity>) -> Self {
        Self(entities)
    }

    pub(crate) fn to_vec(&self) -> Vec<Entity> {
        self.iter().copied().collect()
    }
}

impl Deref for Children {
    type Target = BTreeSet<Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Children {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> IntoIterator for &'a Children {
    type Item = <Self::IntoIter as Iterator>::Item;

    type IntoIter = std::collections::btree_set::Iter<'a, Entity>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
