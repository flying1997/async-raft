use types::pbft::consensus::ReceiveError;
use types::pbft::consensus::SendError;

/*
 * Copyright 2017 Intel Corporation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * -----------------------------------------------------------------------------
 */
use crate::protos::validator::Message;
use crate::protos::validator::Message_MessageType;
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// A Message Sender
///
/// A message
pub trait MessageSender {
    fn send(
        &self,
        destination: Message_MessageType,
        correlation_id: &str,
        contents: &[u8],
    ) -> Result<MessageFuture, SendError>;

    fn reply(
        &self,
        destination: Message_MessageType,
        correlation_id: &str,
        contents: &[u8],
    ) -> Result<(), SendError>;

    fn close(&mut self);
}

/// Result for a message received.
pub type MessageResult = Result<Message, ReceiveError>;

/// A message Receiver
pub type MessageReceiver = Receiver<MessageResult>;

/// A Message Connection
///
/// This denotes a connection which can create a MessageSender/Receiver pair.
pub trait MessageConnection<MS: MessageSender> {
    fn create(&self) -> (MS, MessageReceiver);
}

/// MessageFuture is a promise for the reply to a sent message on connection.
pub struct MessageFuture {
    inner: Receiver<MessageResult>,
    result: Option<MessageResult>,
}

impl MessageFuture {
    pub fn new(inner: Receiver<MessageResult>) -> Self {
        MessageFuture {
            inner,
            result: None,
        }
    }

    pub fn get(&mut self) -> MessageResult {
        if let Some(ref result) = self.result {
            return result.clone();
        }

        match self.inner.recv() {
            Ok(result) => {
                self.result = Some(result.clone());
                result
            }
            Err(err) => Err(ReceiveError::ChannelError(err)),
        }
    }

    pub fn get_timeout(&mut self, timeout: Duration) -> MessageResult {
        if let Some(ref result) = self.result {
            return result.clone();
        }

        match self.inner.recv_timeout(timeout) {
            Ok(result) => {
                self.result = Some(result.clone());
                result
            }
            Err(_) => Err(ReceiveError::TimeoutError),
        }
    }
}

/// Queue for inbound messages, sent directly to this stream.

#[cfg(test)]
mod tests {

    use std::sync::mpsc::channel;
    use std::thread;

    use crate::messages::validator::Message;
    use crate::messages::validator::Message_MessageType;

    use super::MessageFuture;

    fn make_ping(correlation_id: &str) -> Message {
        let mut message = Message::new();
        message.set_message_type(Message_MessageType::PING_REQUEST);
        message.set_correlation_id(String::from(correlation_id));
        message.set_content(String::from("PING").into_bytes());

        message
    }

    #[test]
    fn future_get() {
        let (tx, rx) = channel();

        let mut fut = MessageFuture::new(rx);

        let t = thread::spawn(move || {
            tx.send(Ok(make_ping("my_test"))).unwrap();
        });

        let msg = fut.get().expect("Should have a message");

        t.join().unwrap();

        assert_eq!(msg, make_ping("my_test"));
    }
}
