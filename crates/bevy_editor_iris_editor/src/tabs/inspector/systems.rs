use common::deps::bevy::prelude::{Local, Res, ResMut};
use common::deps::tokio::sync::mpsc::error::TryRecvError;
use common::error::TransactionError;
use common::interface::{Interface, Transaction};
use common::serde::RemoteEntity;
use common::{state_machine, typematch};

use super::messages::{ComponentQuery, SendingEntityData};
use super::InspectorCache;

struct SelectedEntity(RemoteEntity);

pub(crate) enum StreamState {
    NoConnection,
    Connection(Transaction),
    WaitingForConfirmation(Transaction),
    Selected(Transaction, RemoteEntity),
}

pub(crate) fn setup_message_callbacks(interface: ResMut<Interface>) {
    interface.register_callback::<SendingEntityData>(|msg, commands| commands.add(|world| {
        let cache = world.resource_mut::<InspectorCache>();
        if msg.entity == cache.selected() {
            world.resource_mut::<SelectedEntity>().0 = msg.entity;
        }
    }));
}

pub(crate) fn collect_selected_components(
    mut cache: ResMut<InspectorCache>,
    interface: Res<Interface>,
    mut local_state: Local<Option<StreamState>>,
) {
    let state = local_state.take().unwrap();

    // let state = state_machine!(
    //     extern state = StreamState,
    //     run state => {
    //         NoConnection => match interface.open_transaction() {
    //             Ok(trans) => Connection(trans),
    //             Err(_) => break NoConnection,
    //         },
    //         Connection(trans) => match cache.selected() {
    //             &Some(entity) => match trans.send(ComponentQuery { entity }) {
    //                 Ok(()) => WaitingForConfirmation(trans),
    //                 Err(TransactionError::ChannelClosed) => break NoConnection,
    //             },
    //             None => break Connection(trans),
    //         },
    //         WaitingForConfirmation(mut trans) => {
    //             if trans.sender_is_closed() {
    //                 NoConnection
    //             } else {
    //                 let mut selection = None;
    //                 for msg in trans.iter() {
    //                     typematch!(msg.into_any(), {
    //                         msg: SendingEntityData => {
    //                             if &Some(msg.entity) == cache.selected() {
    //                                 selection = Some(msg.entity);
    //                                 break;
    //                             }
    //                         },
    //                         default => (),
    //                     });
    //                 }
    //                 match selection {
    //                     Some(entity) => break Selected(trans, entity),
    //                     None => break WaitingForConfirmation(trans),
    //                 }
    //             }
    //         },
    //         Selected(trans, selected) => {
    //             for msg in trans.iter() {
    //                 typematch!(msg.into_any(), {
    //                     default => ()
    //                 });
    //             }
    //         },
    //     }
    // );

    *local_state = Some(state);

    // loop {
    //     let (mut trans, state) = stream.take().unwrap();
    //     match state {
    //         StreamState::Nothing => match cache.selected() {
    //             &Some(entity) => match trans.send(ComponentQuery { entity }) {
    //                 Ok(()) => *stream = Some((trans, StreamState::Waiting)),
    //                 Err(TransactionError::ChannelClosed) => {
    //                     *stream = None;
    //                     return;
    //                 }
    //             },
    //             None => return,
    //         },
    //         StreamState::Waiting => 'waiting: loop {
    //             match trans.try_recv() {
    //                 Ok(msg) => {
    //                     typematch!(msg.into_any(), {
    //                         msg: SendingEntityData => {
    //                             if &Some(msg.entity) == cache.selected() {
    //                                 *stream = Some((trans, StreamState::Selected(msg.entity)));
    //                                 break 'waiting;
    //                             }
    //                         },
    //                         default => continue,
    //                     });
    //                 }
    //                 Err(TryRecvError::Empty) => return,
    //                 Err(TryRecvError::Disconnected) => {
    //                     *stream = None;
    //                     return;
    //                 }
    //             }
    //         },
    //         StreamState::Selected(selected) => match trans.try_recv() {
    //             Ok(msg) => {
    //                 typematch!(msg.into_any(), {
    //                     default => continue,
    //                 })
    //             }
    //             Err(TryRecvError::Empty) => return,
    //             Err(TryRecvError::Disconnected) => {
    //                 *stream = None;
    //                 return;
    //             }
    //         },
    //     }
    // }
}
