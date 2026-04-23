use gbn_bridge_protocol::BridgeData;

use crate::publisher_client::InProcessPublisherClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardedFrame {
    pub frame: BridgeData,
}

#[derive(Debug, Clone, Default)]
pub struct PayloadForwarder {
    forwarded: Vec<ForwardedFrame>,
}

impl PayloadForwarder {
    pub fn forward(&mut self, publisher_client: &mut InProcessPublisherClient, frame: BridgeData) {
        publisher_client.forward_frame(frame.clone());
        self.forwarded.push(ForwardedFrame { frame });
    }

    pub fn forwarded(&self) -> &[ForwardedFrame] {
        &self.forwarded
    }
}
