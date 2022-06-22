use crate::common::*;
use bevy::{
    reflect::{GetTypeRegistration, TypeRegistry},
    utils::HashMap,
};
use std::any::TypeId;

pub trait RegisterMessage {
    fn register(app: &mut App);
}

impl<M: Message + GetTypeRegistration> RegisterMessage for M {
    fn register(app: &mut App) {
        let registry = app.world.remove_resource::<TypeRegistry>().expect(
            "TypeRegistry not found; insert a TypeRegistry before calling register_messages()",
        );
        let mut inner = registry.write();
        let mut distributor = app.world.remove_resource::<MessageDistributor>().expect(
            "MessageDistributor not found; call app.add_distributor() before registering messages",
        );

        app.add_event::<SendMessage<M>>();
        app.add_event::<MessageReceived<M>>();
        inner.register::<M>();
        distributor.register::<M>();

        drop(inner);
        app.world.insert_resource(registry);
        app.world.insert_resource(distributor);
    }
}

macro_rules! impl_register_message_tuple {
    ($($type:ident),*) => {
        impl<$($type: RegisterMessage),*> RegisterMessage for ($($type),*,) {
            fn register(app: &mut App) {
                $(
                    $type::register(app);
                )*
            }
        }
    }
}

impl_register_message_tuple!(T1);
impl_register_message_tuple!(T1, T2);
impl_register_message_tuple!(T1, T2, T3);
impl_register_message_tuple!(T1, T2, T3, T4);
impl_register_message_tuple!(T1, T2, T3, T4, T5);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_register_message_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

pub trait AppRegisterMsgExt {
    fn add_distributor(&mut self) -> &mut Self;

    fn add_messages<M: RegisterMessage>(&mut self) -> &mut Self;
}

impl AppRegisterMsgExt for App {
    fn add_distributor(&mut self) -> &mut Self {
        self.world.insert_resource(MessageDistributor::default());
        self
    }

    fn add_messages<M: RegisterMessage>(&mut self) -> &mut Self {
        M::register(self);
        self
    }
}

pub fn distribute_messages(world: &mut World) {
    let distributor = world.remove_resource::<MessageDistributor>().unwrap();
    let interface = world.remove_resource::<Interface>().unwrap();

    world.insert_resource(distributor);
    world.insert_resource(interface);
}

#[derive(Deref, DerefMut)]
pub struct MessageReceived<M: Message>(pub M);

#[derive(Deref, DerefMut)]
pub struct SendMessage<M: Message>(pub M);

#[derive(Default)]
pub struct MessageDistributor {
    map: HashMap<TypeId, fn(Box<dyn Message>, &mut World)>,
}

impl MessageDistributor {
    pub fn register<M: Message>(&mut self) {
        self.map.insert(TypeId::of::<M>(), distribute::<M>);
    }

    pub fn distribute(
        &self,
        msg: Box<dyn Message>,
        world: &mut World,
    ) -> Result<(), Box<dyn Message>> {
        if let Some(&distribute_fn) = self.map.get(&msg.type_id()) {
            (distribute_fn)(msg, world);
            Ok(())
        } else {
            Err(msg)
        }
    }
}

fn distribute<M: Message>(msg: Box<dyn Message>, world: &mut World) {
    let msg = *msg
        .into_any()
        .downcast::<M>()
        .expect("attempted to distribute invalid message");

    let mut writer = world
        .get_resource_mut::<EventWriter<MessageReceived<M>>>()
        .unwrap();
    writer.send(MessageReceived(msg));
}
