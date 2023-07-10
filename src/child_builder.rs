use std::collections::BTreeSet;

use crate::{Children, HierarchyEvent, Parents};
use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    prelude::Events,
    system::{Command, Commands, EntityCommands},
    world::{EntityMut, World},
};

// Do not use `world.send_event_batch` as it prints error message when the Events are not available in the world,
// even though it's a valid use case to execute commands on a world without events. Loading a GLTF file for example
fn push_events(world: &mut World, events: impl IntoIterator<Item = HierarchyEvent>) {
    if let Some(mut moved) = world.get_resource_mut::<Events<HierarchyEvent>>() {
        moved.extend(events);
    }
}

fn insert_children_unidirectional(world: &mut World, children: &[Entity], parent: Entity) {
    let mut entity_ext = world.entity_mut(parent);
    entity_ext.insert_children_unidirectional(children);
}
fn insert_parent_unidirectional(world: &mut World, child: Entity, parent: Entity) {
    world.entity_mut(child).insert_parent_unidirectional(parent);
}
fn remove_parent_unidirectional(world: &mut World, child: Entity, parent: Entity) {
    world.entity_mut(child).remove_parent_unidirectional(parent);
}
pub(crate) fn remove_children_unidirectional(
    world: &mut World,
    children: &[Entity],
    parent: Entity,
) {
    world
        .entity_mut(parent)
        .remove_children_unidirectional(children);
}

/// Update the [`Parent`] component of the `child`.
/// Removes the `child` from the previous parent's [`Children`].
///
/// Does not update the new parents [`Children`] component.
///
/// Does nothing if `child` was already a child of `parent`.
///
/// Sends [`HierarchyEvent`]'s.
// fn update_old_parent(world: &mut World, child: Entity, parent: Entity, new_parent: Entity) {
//     let previous = update_parent_unidirectional(world, child, parent);
//     if let Some(previous_parent) = previous {
//         // Do nothing if the child was already parented to this entity.
//         if previous_parent == parent {
//             return;
//         }
//         remove_from_children(world, previous_parent, child);

//         push_events(
//             world,
//             [HierarchyEvent::ChildMoved {
//                 child,
//                 previous_parent,
//                 new_parent: parent,
//             }],
//         );
//     } else {
//         push_events(world, [HierarchyEvent::ChildAdded { child, parent }]);
//     }
// }

/// Update the [`Parent`] components of the `children`.
/// Removes the `children` from their previous parent's [`Children`].
///
/// Does not update the new parents [`Children`] component.
///
/// Does nothing for a child if it was already a child of `parent`.
///
/// Sends [`HierarchyEvent`]'s.
// fn update_old_parents(
//     world: &mut World,
//     children: &[Entity],
//     old_parent: Entity,
//     new_parent: Entity,
// ) {
//     let mut events = Vec::with_capacity(children.len());
//     for &child in children {
//         if let Some(previous) = update_parent(world, child, parent) {
//             // Do nothing if the entity already has the correct parent.
//             if parent == previous {
//                 continue;
//             }

//             remove_from_children(world, previous, child);
//             events.push(HierarchyEvent::ChildMoved {
//                 child,
//                 previous_parent: previous,
//                 new_parent: parent,
//             });
//         } else {
//             events.push(HierarchyEvent::ChildAdded { child, parent });
//         }
//     }
//     push_events(world, events);
// }

/// Removes entities in `children` from `parent`'s [`Children`], removing the component if it ends up empty.
/// Also removes [`Parent`] component from `children`.
fn remove_children(parent: Entity, children: &[Entity], world: &mut World) {
    let mut events = Vec::new();
    if let Some(parent_children) = world.get::<Children>(parent) {
        for &child in children {
            if parent_children.contains(&child) {
                events.push(HierarchyEvent::ChildRemoved { child, parent });
            }
        }
    } else {
        return;
    }
    for event in &events {
        if let &HierarchyEvent::ChildRemoved { child, .. } = event {
            remove_parent_unidirectional(world, child, parent);
        }
    }
    push_events(world, events);

    remove_children_unidirectional(world, children, parent);
}

/// Input nodes as parents. And removes them in [Children] of nodes.
fn clear_children_relation(nodes: &[Entity], world: &mut World) {
    let children = nodes
        .iter()
        .filter_map(|node| world.entity_mut(*node).take::<Children>())
        .flat_map(|children| children.0.into_iter())
        .collect::<BTreeSet<Entity>>();

    for &child in children.iter() {
        let mut entity_ext = world.entity_mut(child);
        if let Some(mut parents_component) = entity_ext.get_mut::<Parents>() {
            for node in nodes {
                parents_component.remove(node);
            }
            if parents_component.is_empty() {
                entity_ext.remove::<Parents>();
            }
        }
    }
}
/// Input nodes as children. And removes them in [Parents] of nodes.
fn clear_parents_relation(nodes: &[Entity], world: &mut World) {
    let parents = nodes
        .iter()
        .filter_map(|node| world.entity_mut(*node).take::<Parents>())
        .flat_map(|node| node.0.into_iter())
        .collect::<BTreeSet<Entity>>();

    for &parent in parents.iter() {
        let mut entity_ext = world.entity_mut(parent);
        if let Some(mut parents_component) = entity_ext.get_mut::<Children>() {
            for node in nodes {
                parents_component.remove(node);
            }
            if parents_component.is_empty() {
                entity_ext.remove::<Parents>();
            }
        }
    }
}

/// Command that adds a child to an entity.
#[derive(Debug)]
pub struct AddChild {
    /// Parent entity to add the child to.
    pub parent: Entity,
    /// Child entity to add.
    pub child: Entity,
}

impl Command for AddChild {
    fn apply(self, world: &mut World) {
        world.entity_mut(self.parent).add_child(self.child);
    }
}
/// Command that moves a child to an entity.
#[derive(Debug)]
pub struct MoveChild {
    /// Parent entity to be moved.
    pub parent: Entity,
    /// Child entity to add.
    pub child: Entity,
    /// Parent entity to move the child to.
    pub new_parent: Entity,
}

impl Command for MoveChild {
    fn apply(self, world: &mut World) {
        world
            .entity_mut(self.parent)
            .move_child(self.new_parent, self.child);
    }
}

/// Command that pushes children to the end of the entity's [`Children`].
#[derive(Debug)]
pub struct PushChildren {
    parent: Entity,
    children: Vec<Entity>,
}

impl Command for PushChildren {
    fn apply(self, world: &mut World) {
        world.entity_mut(self.parent).push_children(&self.children);
    }
}
/// Command that moves children to the end of the entity's [`Children`].
#[derive(Debug)]
pub struct MoveChildren {
    parent: Entity,
    children: Vec<Entity>,
    new_parent: Entity,
}

impl Command for MoveChildren {
    fn apply(self, world: &mut World) {
        world
            .entity_mut(self.parent)
            .move_children(self.new_parent, &self.children);
    }
}

/// Command that removes children from an entity, and removes these children's parent.
pub struct RemoveChildren {
    parent: Entity,
    children: Vec<Entity>,
}

impl Command for RemoveChildren {
    fn apply(self, world: &mut World) {
        remove_children(self.parent, &self.children, world);
    }
}

/// Command that clears all children from an entity and removes [`Parent`] component from those
/// children.
pub struct ClearChildren {
    parent: Entity,
}

impl Command for ClearChildren {
    fn apply(self, world: &mut World) {
        let entity_ext = world.entity_mut(self.parent);
        let children = entity_ext
            .get::<Children>()
            .map(|children| children.to_vec())
            .unwrap_or_default();
        remove_children(self.parent, children.as_slice(), world);
    }
}

/// Command that clear all children from an entity, replacing them with the given children.
pub struct ReplaceChildren {
    parent: Entity,
    children: Vec<Entity>,
}

impl Command for ReplaceChildren {
    fn apply(self, world: &mut World) {
        let mut entity_ext = world.entity_mut(self.parent);
        let replace = BTreeSet::from_iter(self.children);
        let parents = entity_ext.get::<Children>().unwrap();
        let remove_different = parents
            .difference(&replace)
            .copied()
            .collect::<Vec<Entity>>();
        let insert_different = replace
            .difference(parents)
            .copied()
            .collect::<Vec<Entity>>();

        entity_ext.remove_children(remove_different.as_slice());
        entity_ext.push_children(insert_different.as_slice());
    }
}

// /// Command that removes the parent of an entity, and removes that entity from the parent's [`Children`].
// pub struct RemoveParents {
//     /// `Entity` whose parent must be removed.
//     pub child: Entity,
// }

// impl Command for RemoveParents {
//     fn apply(self, world: &mut World) {
//         world.entity_mut(self.child).remove_parents();
//     }
// }
/// Command that removes the parent of an entity, and removes that entity from the parent's [`Children`].
pub struct RemoveParent {
    /// `Entity` whose parent must be removed.
    pub child: Entity,
    /// parent would to be removed.
    pub parent: Entity,
}

impl Command for RemoveParent {
    fn apply(self, world: &mut World) {
        world.entity_mut(self.child).remove_parent(self.parent);
    }
}

/// Struct for building children entities and adding them to a parent entity.
pub struct ChildBuilder<'w, 's, 'a> {
    commands: &'a mut Commands<'w, 's>,
    push_children: PushChildren,
}

impl<'w, 's, 'a> ChildBuilder<'w, 's, 'a> {
    /// Spawns an entity with the given bundle and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityCommands<'w, 's, '_> {
        let e = self.commands.spawn(bundle);
        self.push_children.children.push(e.id());
        e
    }

    /// Spawns an [`Entity`] with no components and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn_empty(&mut self) -> EntityCommands<'w, 's, '_> {
        let e = self.commands.spawn_empty();
        self.push_children.children.push(e.id());
        e
    }

    /// Returns the parent entity of this [`ChildBuilder`].
    pub fn parent_entity(&self) -> Entity {
        self.push_children.parent
    }

    /// Adds a command to be executed, like [`Commands::add`].
    pub fn add_command<C: Command + 'static>(&mut self, command: C) -> &mut Self {
        self.commands.add(command);
        self
    }
}

/// Trait for removing, adding and replacing children and parents of an entity.
pub trait BuildChildren {
    /// Takes a clousre which builds children for this entity using [`ChildBuilder`].
    fn with_children(&mut self, f: impl FnOnce(&mut ChildBuilder)) -> &mut Self;
    /// Pushes children to the back of the builder's children. For any entities that are
    /// already a child of this one, this method does nothing.
    fn push_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Moves children to the back of the builder's children. For any entities that are
    /// already a child of this one, this method does nothing.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    fn move_children(&mut self, new_parent: Entity, children: &[Entity]) -> &mut Self;
    /// Removes the given children
    ///
    /// Removing all children from a parent causes its [`Children`] component to be removed from the entity.
    fn remove_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Adds a single child.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    fn add_child(&mut self, child: Entity) -> &mut Self;
    /// Moves a single child.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    fn move_child(&mut self, new_parent: Entity, child: Entity) -> &mut Self;
    /// Removes all children from this entity. The [`Children`] component will be removed if it exists, otherwise this does nothing.
    fn clear_children(&mut self) -> &mut Self;
    /// Removes all current children from this entity, replacing them with the specified list of entities.
    ///
    /// The removed children will have their [`Parent`] component removed.
    fn replace_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Sets the parent of this entity.
    ///
    /// If this entity already had a parent, the parent's [`Children`] component will have this
    /// child removed from its list. Removing all children from a parent causes its [`Children`]
    /// component to be removed from the entity.
    fn set_parent(&mut self, parent: Entity) -> &mut Self;
    /// Removes the [`Parent`] of this entity.
    ///
    /// Also removes this entity from its parent's [`Children`] component. Removing all children from a parent causes
    /// its [`Children`] component to be removed from the entity.
    fn remove_parent(&mut self, parent: Entity) -> &mut Self;
}

impl<'w, 's, 'a> BuildChildren for EntityCommands<'w, 's, 'a> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut ChildBuilder)) -> &mut Self {
        let parent = self.id();
        let mut builder = ChildBuilder {
            commands: self.commands(),
            push_children: PushChildren {
                children: Vec::default(),
                parent,
            },
        };

        spawn_children(&mut builder);
        let children = builder.push_children;
        self.commands().add(children);
        self
    }

    fn push_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(PushChildren {
            children: Vec::from(children),
            parent,
        });
        self
    }
    fn move_children(&mut self, new_parent: Entity, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(MoveChildren {
            children: Vec::from(children),
            new_parent,
            parent,
        });
        self
    }

    fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(RemoveChildren {
            children: Vec::from(children),
            parent,
        });
        self
    }

    fn move_child(&mut self, new_parent: Entity, child: Entity) -> &mut Self {
        let parent = self.id();
        self.commands().add(MoveChild {
            child,
            parent,
            new_parent,
        });
        self
    }
    fn add_child(&mut self, child: Entity) -> &mut Self {
        let parent = self.id();
        self.commands().add(AddChild { child, parent });
        self
    }

    fn clear_children(&mut self) -> &mut Self {
        let parent = self.id();
        self.commands().add(ClearChildren { parent });
        self
    }

    fn replace_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(ReplaceChildren {
            children: Vec::from(children),
            parent,
        });
        self
    }

    fn set_parent(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();
        self.commands().add(AddChild { child, parent });
        self
    }

    fn remove_parent(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();
        self.commands().add(RemoveParent { child, parent });
        self
    }
}

/// Struct for adding children to an entity directly through the [`World`] for use in exclusive systems.
#[derive(Debug)]
pub struct WorldChildBuilder<'w> {
    world: &'w mut World,
    parent: Entity,
}

impl<'w> WorldChildBuilder<'w> {
    /// Spawns an entity with the given bundle and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parents`] component to the created entity.
    pub fn spawn(&mut self, bundle: impl Bundle + Send + Sync + 'static) -> EntityMut<'_> {
        // insert_parent_unidirectional(self.world, entity, self.parent);
        let entity = self
            .world
            .spawn((bundle, Parents(BTreeSet::from([self.parent]))))
            .id();
        insert_children_unidirectional(self.world, &[entity], self.parent);

        push_events(
            self.world,
            [HierarchyEvent::ChildAdded {
                child: entity,
                parent: self.parent,
            }],
        );
        self.world.entity_mut(entity)
    }

    /// Spawns an [`Entity`] with no components and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn_empty(&mut self) -> EntityMut<'_> {
        let entity = self
            .world
            .spawn(Parents(BTreeSet::from([self.parent])))
            .id();
        insert_children_unidirectional(self.world, &[entity], self.parent);
        push_events(
            self.world,
            [HierarchyEvent::ChildAdded {
                child: entity,
                parent: self.parent,
            }],
        );
        self.world.entity_mut(entity)
    }

    /// Returns the parent entity of this [`WorldChildBuilder`].
    pub fn parent_entity(&self) -> Entity {
        self.parent
    }
}

/// Trait that defines adding, changing and children and parents of an entity directly through the [`World`].
pub trait BuildWorldChildren {
    /// Takes a clousre which builds children for this entity using [`WorldChildBuilder`].
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self;

    /// Moves a single child.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    fn move_child(&mut self, new_parent: Entity, child: Entity) -> &mut Self;

    /// Adds a single child.
    /// Notice that self is parent.
    fn add_child(&mut self, child: Entity) -> &mut Self;

    /// Moves children to the back of the builder's children. For any entities that are
    /// already a child of this one, this method does nothing.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    fn move_children(&mut self, new_parent: Entity, children: &[Entity]) -> &mut Self;

    /// Pushes children to the back of the builder's children. For any entities that are
    /// already a child of this one, this method does nothing.
    fn push_children(&mut self, children: &[Entity]) -> &mut Self;

    // /// Inserts children at the given index.
    // fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;

    /// Removes the given children
    ///
    /// Removing all children from a parent causes its [`Children`] component to be removed from the entity.
    fn remove_children(&mut self, children: &[Entity]) -> &mut Self;

    /// Sets the parent of this entity.
    ///
    /// If this entity already had a parent, the parent's [`Children`] component will have this
    /// child removed from its list. Removing all children from a parent causes its [`Children`]
    /// component to be removed from the entity.
    fn set_parent(&mut self, parent: Entity) -> &mut Self;

    /// Removes the [`Parent`] of this entity.
    ///
    /// Also removes this entity from its parent's [`Children`] component. Removing all children from a parent causes
    /// its [`Children`] component to be removed from the entity.
    fn remove_parent(&mut self, parent: Entity) -> &mut Self;
    /// despawn a node.
    fn clear(self);
}

impl<'w> BuildWorldChildren for EntityMut<'w> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            spawn_children(&mut WorldChildBuilder { world, parent });
        });
        self
    }

    fn move_child(&mut self, new_parent: Entity, child: Entity) -> &mut Self {
        let parent = self.id();

        self.remove_children_unidirectional(&[child]);

        self.world_scope(|world| {
            // let mut entity_ext = world.entity_mut(new_parent);
            // if let Some(mut children_component) = entity_ext.get_mut::<Children>() {
            //     children_component.insert(child);
            // } else {
            //     entity_ext.insert(Children::new(BTreeSet::from([child])));
            // }
            insert_children_unidirectional(world, &[child], new_parent);

            // handle Parent
            let mut entity_ext = world.entity_mut(child);
            if let Some(mut parents_component) = entity_ext.get_mut::<Parents>() {
                parents_component.remove(&parent);
                parents_component.insert(new_parent);
            } else {
                entity_ext.insert(Parents::new(BTreeSet::from([new_parent])));
            }
        });

        self
    }

    fn add_child(&mut self, child: Entity) -> &mut Self {
        let parent = self.id();

        self.insert_children_unidirectional(&[child]);
        self.world_scope(|world| {
            insert_parent_unidirectional(world, child, parent);
            push_events(world, [HierarchyEvent::ChildAdded { child, parent }]);
        });

        self
    }

    fn move_children(&mut self, new_parent: Entity, children: &[Entity]) -> &mut Self {
        let parent = self.id();

        self.remove_children_unidirectional(children);
        self.world_scope(|world| {
            insert_children_unidirectional(world, children, new_parent);

            // handle Parent
            for &child in children {
                let mut entity_ext = world.entity_mut(child);
                if let Some(mut parents_component) = entity_ext.get_mut::<Parents>() {
                    parents_component.remove(&parent);
                    parents_component.insert(new_parent);
                } else {
                    entity_ext.insert(Parents::new(BTreeSet::from([new_parent])));
                }
            }
        });
        self
    }
    fn push_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        let mut child_vec = Vec::with_capacity(children.len());

        let children = if let Some(children_component) = self.get_mut::<Children>() {
            child_vec.extend(
                children
                    .iter()
                    .filter(|child| !children_component.contains(child))
                    .copied(),
            );
            child_vec.as_slice()
        } else {
            children
        };
        let events = children
            .iter()
            .map(|child| HierarchyEvent::ChildAdded {
                child: *child,
                parent,
            })
            .collect::<Vec<_>>();

        self.insert_children_unidirectional(children);

        self.world_scope(|world| {
            for &child in children {
                insert_parent_unidirectional(world, child, parent);
            }

            push_events(world, events);
        });
        self
    }

    fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();

        self.world_scope(|world| {
            // This is complicated, there are early abort.
            remove_children(parent, children, world);
        });
        self
    }

    fn set_parent(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();
        self.insert_parent_unidirectional(parent);

        self.world_scope(|world| {
            insert_children_unidirectional(world, &[child], parent);
            push_events(world, [HierarchyEvent::ChildAdded { child, parent }]);
        });
        self
    }

    fn remove_parent(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();

        self.remove_parent_unidirectional(parent);
        self.world_scope(|world| {
            remove_children_unidirectional(world, &[child], parent);
            push_events(world, [HierarchyEvent::ChildRemoved { child, parent }]);
        });

        self
    }

    fn clear(mut self) {
        let node = self.id();
        self.world_scope(|world| {
            clear_children_relation(&[node], world);
            clear_parents_relation(&[node], world);
        });
        self.despawn();
    }
}

/// [UnidirectionalExt] is used for [EntityMut]
trait UnidirectionalExt {
    fn insert_children_unidirectional(&mut self, children: &[Entity]);

    fn remove_children_unidirectional(&mut self, children: &[Entity]);

    fn insert_parent_unidirectional(&mut self, parent: Entity);

    fn remove_parent_unidirectional(&mut self, parent: Entity);
}
impl UnidirectionalExt for EntityMut<'_> {
    fn insert_children_unidirectional(&mut self, children: &[Entity]) {
        if let Some(mut children_component) = self.get_mut::<Children>() {
            children_component.extend(children);
        } else {
            self.insert(Children::new(BTreeSet::from_iter(children.iter().copied())));
        }
    }

    fn remove_children_unidirectional(&mut self, children: &[Entity]) {
        if let Some(mut children_component) = self.get_mut::<Children>() {
            for child in children {
                children_component.remove(child);
            }
            if children_component.is_empty() {
                self.remove::<Children>();
            }
        }
    }

    fn insert_parent_unidirectional(&mut self, parent: Entity) {
        if let Some(mut parents_component) = self.get_mut::<Parents>() {
            parents_component.insert(parent);
        } else {
            self.insert(Parents::new(BTreeSet::from([parent])));
        }
    }

    fn remove_parent_unidirectional(&mut self, parent: Entity) {
        if let Some(mut parents_component) = self.get_mut::<Parents>() {
            parents_component.remove(&parent);
            if parents_component.is_empty() {
                self.remove::<Parents>();
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{BuildChildren, BuildWorldChildren};
    use crate::{
        components::{Children, Parents},
        HierarchyEvent::{self, ChildAdded, ChildRemoved},
    };

    use bevy_ecs::{
        component::Component,
        entity::Entity,
        event::Events,
        system::{CommandQueue, Commands},
        world::World,
    };

    /// Assert the (non)existence and state of the child's [`Parent`] component.
    fn assert_parents(world: &mut World, child: Entity, parent: &[Entity]) {
        assert_eq!(
            world
                .get::<Parents>(child)
                .into_iter()
                .flatten()
                .copied()
                .collect::<Vec<_>>(),
            parent
        );
    }

    /// Assert the (non)existence and state of the parent's [`Children`] component.
    fn assert_children(world: &mut World, parent: Entity, children: &[Entity]) {
        assert_eq!(
            world
                .get::<Children>(parent)
                .unwrap()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            children
        );
    }

    /// Used to omit a number of events that are not relevant to a particular test.
    fn omit_events(world: &mut World, number: usize) {
        let mut events_resource = world.resource_mut::<Events<HierarchyEvent>>();
        let mut events: Vec<_> = events_resource.drain().collect();
        events_resource.extend(events.drain(number..));
    }

    fn assert_events(world: &mut World, expected_events: &[HierarchyEvent]) {
        let events: Vec<_> = world
            .resource_mut::<Events<HierarchyEvent>>()
            .drain()
            .collect();
        assert_eq!(events, expected_events);
    }

    #[test]
    fn add_child() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).add_child(b);

        assert_parents(world, b, &[a]);
        assert_children(world, a, &[b]);
        assert_events(
            world,
            &[ChildAdded {
                child: b,
                parent: a,
            }],
        );

        world.entity_mut(a).add_child(c);

        assert_children(world, a, &[b, c]);
        assert_parents(world, c, &[a]);
        assert_events(
            world,
            &[ChildAdded {
                child: c,
                parent: a,
            }],
        );
        // Children component should be removed when it's empty.
        world.entity_mut(a).remove_children(&[b, c]);
        assert!(world.get::<Children>(a).is_none());
    }

    #[test]
    fn set_parent() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).set_parent(b);

        assert_parents(world, a, &[b]);
        assert_children(world, b, &[a]);
        assert_events(
            world,
            &[ChildAdded {
                child: a,
                parent: b,
            }],
        );

        world.entity_mut(a).set_parent(c);

        assert_parents(world, a, &[b, c]);
        assert_children(world, b, &[a]);
        assert_children(world, c, &[a]);
        assert_events(
            world,
            &[ChildAdded {
                child: a,
                parent: c,
            }],
        );
    }

    // regression test for https://github.com/bevyengine/bevy/pull/8346
    #[test]
    fn set_parent_of_orphan() {
        let world = &mut World::new();

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());
        world.entity_mut(a).set_parent(b);
        assert_parents(world, a, &[b]);
        assert_children(world, b, &[a]);

        world.entity_mut(b).clear();

        world.entity_mut(a).set_parent(c);

        assert_parents(world, a, &[c]);
        assert_children(world, c, &[a]);
    }

    #[test]
    fn remove_parent() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).push_children(&[b, c]);
        world.entity_mut(b).remove_parent(a);

        assert!(world.get::<Parents>(b).is_none());

        assert_parents(world, c, &[a]);
        assert_children(world, a, &[c]);
        omit_events(world, 2);
        // Omit ChildAdded events.
        assert_events(
            world,
            &[ChildRemoved {
                child: b,
                parent: a,
            }],
        );

        world.entity_mut(c).remove_parent(a);
        assert!(world.get::<Parents>(c).is_none());
        assert!(world.get::<Children>(a).is_none());
        assert_events(
            world,
            &[ChildRemoved {
                child: c,
                parent: a,
            }],
        );
    }

    #[derive(Component)]
    struct C(u32);

    #[test]
    fn build_children() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let parent = commands.spawn(C(1)).id();
        let mut children = Vec::new();
        commands.entity(parent).with_children(|parent| {
            children.extend([
                parent.spawn(C(2)).id(),
                parent.spawn(C(3)).id(),
                parent.spawn(C(4)).id(),
            ]);
        });

        queue.apply(&mut world);
        assert_eq!(
            world
                .get::<Children>(parent)
                .unwrap()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            children,
        );
        assert_eq!(
            *world.get::<Parents>(children[0]).unwrap().first().unwrap(),
            parent
        );
        assert_eq!(
            *world.get::<Parents>(children[1]).unwrap().first().unwrap(),
            parent
        );

        assert_eq!(
            *world.get::<Parents>(children[0]).unwrap().first().unwrap(),
            parent
        );
        assert_eq!(
            *world.get::<Parents>(children[1]).unwrap().first().unwrap(),
            parent
        );
    }

    #[test]
    fn push_and_insert_and_remove_children_commands() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3), C(4), C(5)])
            .collect::<Vec<Entity>>();

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(entities[0]).push_children(&entities[1..3]);
        }
        queue.apply(&mut world);

        let parent = entities[0];
        let child1 = entities[1];
        let child2 = entities[2];
        let child3 = entities[3];
        let child4 = entities[4];

        let expected_children: Vec<Entity> = vec![child1, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().to_vec(),
            expected_children
        );
        assert_eq!(
            *world.get::<Parents>(child1).unwrap().first().unwrap(),
            parent
        );
        assert_eq!(
            *world.get::<Parents>(child2).unwrap().first().unwrap(),
            parent
        );
        assert_eq!(
            *world.get::<Parents>(child1).unwrap().first().unwrap(),
            parent
        );
        assert_eq!(
            *world.get::<Parents>(child2).unwrap().first().unwrap(),
            parent
        );

        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent).push_children(&entities[3..]);
        }
        queue.apply(&mut world);

        let expected_children: Vec<Entity> = vec![child1, child2, child3, child4];
        assert_eq!(
            world.get::<Children>(parent).unwrap().to_vec(),
            expected_children
        );
        assert_eq!(
            *world.get::<Parents>(child3).unwrap().to_vec(),
            vec![parent]
        );
        assert_eq!(
            *world.get::<Parents>(child4).unwrap().to_vec(),
            vec![parent]
        );

        let remove_children = [child1, child4];
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent).remove_children(&remove_children);
        }
        queue.apply(&mut world);

        let expected_children: Vec<Entity> = vec![child2, child3];
        assert_eq!(
            world.get::<Children>(parent).unwrap().to_vec(),
            expected_children
        );
        assert!(world.get::<Parents>(child1).is_none());
        assert!(world.get::<Parents>(child4).is_none());
    }

    #[test]
    fn push_and_clear_children_commands() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![(C(1),), (C(2),), (C(3),), (C(4),), (C(5),)])
            .collect::<Vec<Entity>>();

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(entities[0]).push_children(&entities[1..3]);
        }
        queue.apply(&mut world);

        let parent = entities[0];
        let child1 = entities[1];
        let child2 = entities[2];

        let expected_children: Vec<Entity> = vec![child1, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().to_vec(),
            expected_children
        );
        assert_eq!(
            *world.get::<Parents>(child1).unwrap().to_vec(),
            vec![parent]
        );
        assert_eq!(
            *world.get::<Parents>(child2).unwrap().to_vec(),
            vec![parent]
        );

        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent).clear_children();
        }
        queue.apply(&mut world);

        assert!(world.get::<Children>(parent).is_none());
        assert!(world.get::<Parents>(child1).is_none());
        assert!(world.get::<Parents>(child2).is_none());
    }

    #[test]
    fn push_and_replace_children_commands() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![(C(1),), (C(2),), (C(3),), (C(4),), (C(5),)])
            .collect::<Vec<Entity>>();

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(entities[0]).push_children(&entities[1..3]);
        }
        queue.apply(&mut world);

        let parent = entities[0];
        let child1 = entities[1];
        let child2 = entities[2];
        let child4 = entities[4];

        let expected_children: Vec<Entity> = vec![child1, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().to_vec(),
            expected_children
        );
        assert_eq!(
            *world.get::<Parents>(child1).unwrap().to_vec(),
            vec![parent]
        );
        assert_eq!(
            *world.get::<Parents>(child2).unwrap().to_vec(),
            vec![parent]
        );

        let replace_children = [child1, child4];
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent).replace_children(&replace_children);
        }
        queue.apply(&mut world);

        let expected_children: Vec<Entity> = vec![child1, child4];
        assert_eq!(
            world.get::<Children>(parent).unwrap().to_vec(),
            expected_children
        );
        assert_eq!(
            *world.get::<Parents>(child1).unwrap().to_vec(),
            vec![parent]
        );
        assert_eq!(
            *world.get::<Parents>(child4).unwrap().to_vec(),
            vec![parent]
        );
        assert!(world.get::<Parents>(child2).is_none());
    }

    #[test]
    fn push_and_insert_and_remove_children_world() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3), C(4), C(5)])
            .collect::<Vec<Entity>>();

        world.entity_mut(entities[0]).push_children(&entities[1..3]);

        let parent = entities[0];
        let child1 = entities[1];
        let child2 = entities[2];
        let child3 = entities[3];
        let child4 = entities[4];

        let expected_children: Vec<Entity> = vec![child1, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().to_vec(),
            expected_children
        );
        assert_eq!(
            *world.get::<Parents>(child1).unwrap().to_vec(),
            vec![parent]
        );
        assert_eq!(
            *world.get::<Parents>(child2).unwrap().to_vec(),
            vec![parent]
        );

        world.entity_mut(parent).push_children(&entities[3..]);
        let expected_children: Vec<Entity> = vec![child1, child2, child3, child4];
        assert_eq!(
            world.get::<Children>(parent).unwrap().to_vec(),
            expected_children
        );
        assert_eq!(
            *world.get::<Parents>(child3).unwrap().to_vec(),
            vec![parent]
        );
        assert_eq!(
            *world.get::<Parents>(child4).unwrap().to_vec(),
            vec![parent]
        );

        let remove_children = [child1, child4];
        world.entity_mut(parent).remove_children(&remove_children);
        let expected_children: Vec<Entity> = vec![child2, child3];
        assert_eq!(
            world.get::<Children>(parent).unwrap().to_vec(),
            expected_children
        );
        assert!(world.get::<Parents>(child1).is_none());
        assert!(world.get::<Parents>(child4).is_none());
    }

    // /// Tests what happens when all children are removed from a parent using world functions
    #[test]
    fn children_removed_when_empty_world() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3)])
            .collect::<Vec<Entity>>();

        let parent1 = entities[0];
        let parent2 = entities[1];
        let child = entities[2];

        // push child into parent1
        world.entity_mut(parent1).push_children(&[child]);
        assert_eq!(
            world.get::<Children>(parent1).unwrap().to_vec().as_slice(),
            &[child]
        );

        // move only child from parent1 with `push_children`
        world.entity_mut(parent1).move_children(parent2, &[child]);
        assert!(world.get::<Children>(parent1).is_none());

        // move only child from parent2 with `push_children`
        world.entity_mut(parent2).move_children(parent1, &[child]);
        assert!(world.get::<Children>(parent2).is_none());

        // remove only child from parent1 with `remove_children`
        world.entity_mut(parent1).remove_children(&[child]);
        assert!(world.get::<Children>(parent1).is_none());
    }

    // /// Tests what happens when all children are removed form a parent using commands
    #[test]
    fn children_removed_when_empty_commands() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3)])
            .collect::<Vec<Entity>>();

        let parent1 = entities[0];
        let parent2 = entities[1];
        let child = entities[2];

        let mut queue = CommandQueue::default();

        // push child into parent1
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent1).push_children(&[child]);
            queue.apply(&mut world);
        }
        assert_eq!(
            world.get::<Children>(parent1).unwrap().to_vec().as_slice(),
            &[child]
        );

        // move only child from parent1 with `push_children`
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent2).push_children(&[child]);
            queue.apply(&mut world);
        }
        assert!(world.get::<Children>(parent1).is_some());

        // move only child from parent2 with `push_children`
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent1).push_children(&[child]);
            queue.apply(&mut world);
        }
        assert_eq!(
            world.get::<Children>(parent2).unwrap().to_vec(),
            vec![child]
        );

        // move only child from parent1 with `add_child`
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent1).move_child(parent2, child);
            queue.apply(&mut world);
        }
        assert!(world.get::<Children>(parent1).is_none());

        // remove only child from parent2 with `remove_children`
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent2).remove_children(&[child]);
            queue.apply(&mut world);
        }
        assert!(world.get::<Children>(parent2).is_none());
    }

    #[test]
    fn regression_push_children_same_archetype() {
        let mut world = World::new();
        let child = world.spawn_empty().id();
        world.spawn_empty().push_children(&[child]);
    }

    #[test]
    fn push_children_idempotent() {
        let mut world = World::new();
        let child = world.spawn_empty().id();
        let parent = world
            .spawn_empty()
            .push_children(&[child])
            .push_children(&[child])
            .id();

        let mut query = world.query::<&Children>();
        let children = query.get(&world, parent).unwrap();
        assert_eq!(children.to_vec(), vec![child]);
    }
}
