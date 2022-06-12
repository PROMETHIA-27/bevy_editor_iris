use std::collections::VecDeque;
use thiserror::Error;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct Interface<In, Out> {
    pub(crate) incoming: Receiver<In>,
    pub(crate) outgoing: Sender<Out>,
    pub(crate) unhandled: VecDeque<In>,
}

impl<In, Out> Interface<In, Out> {
    pub fn new(outgoing: Sender<Out>, incoming: Receiver<In>) -> Self {
        Self {
            outgoing,
            incoming,
            unhandled: Default::default(),
        }
    }

    pub fn wait_for_response<F: FnMut(&In) -> bool>(
        &mut self,
        predicate: F,
    ) -> Result<In, WaitForResponseError> {
        let msg_idx = self
            .unhandled
            .iter()
            .enumerate()
            .find(|(_, msg)| predicate(msg))
            .map(|(i, _)| i);

        if let Some(index) = msg_idx {
            Ok(self.unhandled.remove(index).unwrap())
        } else {
            loop {
                match self.incoming.blocking_recv() {
                    Some(msg) if predicate(&msg) => break Ok(msg),
                    Some(msg) => self.unhandled.push_back(msg),
                    None => break Err(WaitForResponseError::NoMoreMessages),
                }
            }
        }
    }

    pub fn get_unhandled_responses<F: FnMut(&In) -> bool>(&mut self, predicate: F) -> Vec<In> {
        let mut responses = vec![];
        let mut idx = 0;
        while idx < self.unhandled.len() {
            if predicate(&self.unhandled[idx]) {
                responses.push(self.unhandled.remove(idx).unwrap())
            } else {
                idx += 1
            }
        }

        responses
    }
}

#[derive(Debug, Error)]
pub enum WaitForResponseError {
    #[error("no more messages can be received from the interface")]
    NoMoreMessages,
}
