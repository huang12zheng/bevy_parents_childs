//!  clone from [super::children]
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

pub struct Parents(pub(crate) BTreeSet<Entity>);

// TODO: We need to impl either FromWorld or Default so Parents can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However Parents should only ever be set with a real user-defined entities. Its worth looking
// into better ways to handle cases like this.
impl FromWorld for Parents {
    fn from_world(_world: &mut World) -> Self {
        Parents(BTreeSet::new())
    }
}

impl Parents {
    /// Constructs a [`Parents`] component with the given entities.
    pub(crate) fn new(entities: BTreeSet<Entity>) -> Self {
        Self(entities)
    }
    /// switch parents as vec
    pub fn to_vec(&self) -> Vec<Entity> {
        self.iter().copied().collect()
    }
}

impl Deref for Parents {
    type Target = BTreeSet<Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Parents {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> IntoIterator for &'a Parents {
    type Item = <Self::IntoIter as Iterator>::Item;

    type IntoIter = std::collections::btree_set::Iter<'a, Entity>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
